extern crate chrono;
extern crate timer;

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, SyncSender, Sender};
use std::sync::{mpsc};
use std::thread;

use log::{debug, info, warn};
use timer::Guard;
use warp::Filter;

use crate::config::{NotifierSettings, Settings};
use crate::notifier::{NoOpNotifier, Notifier, Alert};
use crate::notifiers::webhook::WebhookNotifier;
use crate::notifiers::slack::SlackNotifier;

mod notifier;

mod notifiers { pub mod webhook; pub mod slack; }

mod config;

fn build_notifier_set(cfx: &Settings) -> Result<Vec<Box<dyn Notifier>>, String> {
    let mut notifiers: Vec<Box<dyn Notifier>> = Vec::new();

    for notifier_setting in cfx.notifiers.iter() {
        match build_notifier(notifier_setting) {
            Ok(notifier) => notifiers.push(notifier),
            Err(e) => {
                return Err(format!("failed to build notifier: {}", e))
            },
        }
    }

    Ok(notifiers)
}

fn build_notifier(cfg: &NotifierSettings) -> Result<Box<dyn Notifier>, String> {
    match cfg.notifier_type.as_str() {
        "webhook" => match cfg.webhook {
            Some(ref wh) => Ok(
                Box::new(WebhookNotifier::new(
                    wh.url.clone(),
                    wh.method.clone(),
                    wh.body.clone(),
                    wh.headers.clone().unwrap_or(vec![])
                )),
            ),
            None => Err("no webhook settings found".to_string()),
        },
        "slack" => match cfg.slack {
            Some(ref wh) => Ok(
                Box::new(SlackNotifier::new(
                    wh.url.clone(),
                    wh.icon_emoji.clone(),
                    wh.color.clone(),
                )),
            ),
            None => Err("no slack settings found".to_string()),
        },
        "noop" => Ok(Box::new(NoOpNotifier {})),
        t => Err(format!("unsupported notifier: {}", t))
    }
}

fn main() {
    env_logger::init();

    let settings = config::retrieve_settings(Some("dodemansknop.yaml")).unwrap();

    info!("loaded settings: {:?}", settings);

    let (tx_ping, rx_ping): (SyncSender<String>, Receiver<String>) = mpsc::sync_channel(32);
    let (tx_alert, rx_alert): (Sender<Alert>, Receiver<Alert>) = mpsc::channel();
    let notifier_set = build_notifier_set(&settings).unwrap();

    run_alerter_thread(rx_alert, notifier_set);
    run_ping_receiver_thread(rx_ping, tx_alert, settings.clone());

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            serve_api(tx_ping).await;
        });
}

fn run_alerter_thread(rx_alert: Receiver<Alert>, notifier_set: Vec<Box<dyn Notifier>>) {
    thread::spawn(move || {
        loop {
            let r = rx_alert.recv();
            if r.is_err() {
                warn!("error while receiving alert: {}", r.err().unwrap());
                continue;
            }

            let alert = r.unwrap();

            for notifier in notifier_set.iter() {
                match notifier.notify_failure(alert.clone()) {
                    Ok(_) => info!("failure notified"),
                    Err(e) => warn!("error while notifying about failure: {}", e)
                }
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

            active_timers.insert(id, timer.schedule_with_delay(delay, move || {
                info!("missed ping for {}; scheduling alert", idc);

                let alert = Alert{
                    id: idc.clone(),
                };

                match tx_cpy.send(alert) {
                    Ok(_) => debug!("alert scheduled for {}", idc),
                    Err(e) => warn!("error while scheduling alert: {}", e)
                }
            }));
        }
    });
}

async fn serve_api(tx_ping: SyncSender<String>) {
    let api = filters::routes(tx_ping);
    let routes = api.with(warp::log("ping"));

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    return;
}

mod filters {
    use std::convert::Infallible;
    use std::sync::mpsc::SyncSender;

    use warp::Filter;

    use super::handlers;

    pub fn routes(tx_ping: SyncSender<String>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        ping(tx_ping).or(health())
    }

    pub fn ping(ping_tx: SyncSender<String>) -> impl Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::path!("ping" / String)
            .and(warp::post())
            .and(with_ping_tx(ping_tx))
            .and_then(handlers::ping)
    }

    pub fn health() -> impl Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::path!("health")
            .and(warp::get())
            .and_then(handlers::health)
    }

    fn with_ping_tx(tx: SyncSender<String>) -> impl Filter<Extract=(SyncSender<String>, ), Error=Infallible> + Clone {
        warp::any().map(move || tx.clone())
    }
}

mod handlers {
    use std::convert::Infallible;
    use std::sync::mpsc::SyncSender;

    use log::warn;
    use warp::http::StatusCode;

    pub async fn ping(id: String, tx: SyncSender<String>) -> Result<impl warp::Reply, Infallible> {
        match tx.send(id) {
            Ok(_) => Ok(StatusCode::OK),
            Err(err) => {
                warn!("error while sending ping to handler thread: {}", err);
                Ok(StatusCode::SERVICE_UNAVAILABLE)
            }
        }
    }

    pub async fn health() -> Result<impl warp::Reply, Infallible> {
        Ok(StatusCode::OK)
    }
}