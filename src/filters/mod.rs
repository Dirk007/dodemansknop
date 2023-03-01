use std::{convert::Infallible, sync::mpsc::SyncSender};

use warp::Filter;

use crate::handlers;

pub fn routes(tx_ping: SyncSender<String>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    ping(tx_ping).or(health())
}

pub fn ping(ping_tx: SyncSender<String>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("ping" / String)
        .and(warp::post())
        .and(with_ping_tx(ping_tx))
        .and_then(handlers::ping)
}

pub fn health() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("health").and(warp::get()).and_then(handlers::health)
}

fn with_ping_tx(tx: SyncSender<String>) -> impl Filter<Extract = (SyncSender<String>,), Error = Infallible> + Clone {
    warp::any().map(move || tx.clone())
}
