use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        mpsc,
        mpsc::{Receiver, Sender, SyncSender},
    },
    thread,
};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
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

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init()
        .ok();

    let args = Arguments::parse();
    let settings = config::retrieve_settings(args.config_file)?;

    info!("loaded settings: {:?}", settings);

    let (tx_ping, rx_ping): (SyncSender<String>, Receiver<String>) = mpsc::sync_channel(32);
    let (tx_alert, rx_alert): (Sender<Alert>, Receiver<Alert>) = mpsc::channel();
    let notifier_set = build_notifier_set(&settings)?;

    run_alerter_thread(rx_alert, notifier_set);
    run_ping_receiver_thread(rx_ping, tx_alert, settings.clone());

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            serve_api(args.listen_addr.unwrap_or(String::from("0.0.0.0:3030")), tx_ping).await;
        });

    Ok(())
}

fn run_alerter_thread(rx_alert: Receiver<Alert>, notifier_set: Vec<Box<dyn Notifier>>) {
    thread::spawn(move || loop {
        let r = rx_alert.recv();
        if r.is_err() {
            warn!("error while receiving alert: {}", r.err().unwrap());
            continue;
        }

        let alert = r.unwrap();

        for notifier in notifier_set.iter() {
            match notifier.notify_failure(alert.clone()) {
                Ok(_) => info!("failure notified"),
                Err(e) => warn!("error while notifying about failure: {}", e),
            }
        }
    });
}

fn run_ping_receiver_thread(rx_ping: Receiver<String>, tx_alert: Sender<Alert>, settings: Settings) {
    thread::spawn(move || {
        let timer = timer::Timer::new();
        let delay = chrono::Duration::seconds(settings.timeout.unwrap_or(30));

        let mut active_timers: HashMap<String, Guard> = HashMap::new();

        loop {
            let r = rx_ping.recv();
            if r.is_err() {
                warn!("error while receiving ping: {}", r.err().unwrap());
                continue;
            }

            let id = r.unwrap();
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
    });
}

async fn serve_api(listen_addr: String, tx_ping: SyncSender<String>) {
    let api = filters::routes(tx_ping);
    let routes = api.with(warp::log("ping"));

    let addr: SocketAddr = listen_addr.parse().unwrap();

    warp::serve(routes).run(addr).await;
    return;
}
