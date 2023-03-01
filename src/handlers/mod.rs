use std::{convert::Infallible, sync::mpsc::SyncSender};

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
