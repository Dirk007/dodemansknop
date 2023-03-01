use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        mpsc,
        mpsc::{Receiver, Sender, SyncSender},
    },
};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use log::{debug, info, warn};
use timer::Guard;
use warp::Filter;

use crate::{
    config::{NotifierSettings, Settings},
    notifier::{Alert, NoOpNotifier, Notifier},
    notifiers::{slack::SlackNotifier, webhook::WebhookNotifier},
};

mod filters;
mod handlers;
mod notifier;

mod notifiers {
    pub mod slack;
    pub mod webhook;
}

mod config;

#[derive(Parser, Default, Debug)]
#[command(author = "Martin Helmich <m.helmich@mittwald.de>", version, about = "A simple dead mans switch")]
struct Arguments {
    #[arg(short, long = "config")]
    /// Path to the configuration file
    config_file: Option<String>,

    #[arg(short, long)]
    /// Address to bind to
    listen_addr: Option<String>,
}

fn build_notifier_set(cfx: &Settings) -> Result<Vec<Box<dyn Notifier>>> {
    let mut notifiers: Vec<Box<dyn Notifier>> = Vec::with_capacity(cfx.notifiers.len());

    for notifier_setting in cfx.notifiers.iter() {
        notifiers.push(build_notifier(notifier_setting).with_context(|| format!("failed to build notifier:  {:?}", notifier_setting))?);
    }

    Ok(notifiers)
}

fn build_notifier(cfg: &NotifierSettings) -> Result<Box<dyn Notifier>> {
    match cfg.notifier_type.as_str() {
        "webhook" => {
            let wh = cfg.webhook.as_ref().ok_or_else(|| anyhow!("no webhook settings found"))?;
            Ok(Box::new(WebhookNotifier::new(
                wh.url.clone(),
                wh.method.clone(),
                wh.body.clone(),
                wh.headers.clone().unwrap_or(vec![]),
            )))
        }
        "slack" => {
            let wh = cfg.slack.as_ref().ok_or_else(|| anyhow!("no slack settings found"))?;
            Ok(Box::new(SlackNotifier::new(
                wh.url.clone(),
                wh.icon_emoji.clone(),
                wh.color.clone(),
            )))
        }
        "noop" => Ok(Box::new(NoOpNotifier {})),
        _ => bail!("unknown notifier type: {}", cfg.notifier_type),
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init()
        .ok();

    let args = Arguments::parse();
    let settings = config::retrieve_settings(args.config_file)?;

    info!("loaded settings: {:?}", settings);

    let (tx_ping, rx_ping) = mpsc::sync_channel::<String>(32);
    let (tx_alert, rx_alert) = mpsc::channel::<Alert>();
    let notifier_set = build_notifier_set(&settings)?;

    let mut tasks: FuturesUnordered<tokio::task::JoinHandle<Result<()>>> = FuturesUnordered::new();
    tasks.push(tokio::spawn(run_alerter_thread(rx_alert, notifier_set)));
    tasks.push(tokio::spawn(run_ping_receiver_thread(rx_ping, tx_alert, settings.clone())));
    tasks.push(tokio::spawn(serve_api(
        args.listen_addr.unwrap_or(String::from("0.0.0.0:3030")),
        tx_ping,
    )));

    // FTR: This spins up all tasks. If one of the tasks dies (errors) then we just want to exit.
    let died = tasks.next().await;
    log::error!("One of the tasks died {:?}. Exiting.", died);

    Ok(())
}

async fn run_alerter_thread(rx_alert: Receiver<Alert>, notifier_set: Vec<Box<dyn Notifier>>) -> Result<()> {
    loop {
        let alert = rx_alert.recv()?;

        for notifier in notifier_set.iter() {
            match notifier.notify_failure(alert.clone()) {
                Ok(_) => info!("failure notified"),
                Err(e) => warn!("error while notifying about failure: {}", e),
            }
        }
    }
}

async fn run_ping_receiver_thread(rx_ping: Receiver<String>, tx_alert: Sender<Alert>, settings: Settings) -> Result<()> {
    let timer = timer::Timer::new();
    let delay = chrono::Duration::seconds(settings.timeout.unwrap_or_else(|| 30));

    let mut active_timers: HashMap<String, Guard> = HashMap::new();

    loop {
        let id = rx_ping.recv()?;

        let idc = id.clone();

        let tx_cpy = tx_alert.clone();

        debug!("received ping for {}; timeout is {}", id, delay);

        active_timers.insert(
            id,
            timer.schedule_with_delay(delay, move || {
                info!("missed ping for {}; scheduling alert", idc);

                let alert = Alert { id: idc.clone() };

                match tx_cpy.send(alert) {
                    Ok(_) => debug!("alert scheduled for {}", idc),
                    Err(e) => warn!("error while scheduling alert: {}", e),
                }
            }),
        );
    }
}

async fn serve_api(listen_addr: String, tx_ping: SyncSender<String>) -> Result<()> {
    let api = filters::routes(tx_ping);
    let routes = api.with(warp::log("ping"));

    let addr: SocketAddr = listen_addr.parse()?;

    warp::serve(routes).run(addr).await;

    Ok(())
}
