use log::debug;
use reqwest::blocking::Client;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::notifier::{Notifier, Alert};

#[derive(Clone)]
pub struct WebhookNotifier {
    url: String,
    method: String,
    headers: Vec<(String, String)>,

    client: Client,
}

#[derive(Serialize, Deserialize)]
pub struct WebhookMessage {
    id: String,
    service: String,
    severity: String,
    title: String,
    message: String,
}

impl WebhookNotifier {
    pub fn new(url: String, method: String, headers: Vec<(String, String)>) -> Self {
        Self {
            url,
            method,
            headers,
            client: Client::new(),
        }
    }
}

impl Notifier for WebhookNotifier {
    fn notify_failure(&self, alert: Alert) -> Result<(), &'static str> {
        let method = match self.method.to_lowercase().as_str() {
            "get" => Method::GET,
            "post" => Method::POST,
            "put" => Method::PUT,
            _ => Method::GET,
        };

        let msg = WebhookMessage{
            id: alert.id.clone(),
            message: format!("service {} missed its dead-man-switch", alert.id),
            service: alert.id.clone(),
            severity: alert.severity,
            title: String::from("Dead man switch missed"),
        };

        let mut rb = self.client.request(method, &self.url).json(&msg);

        for (header, value) in self.headers.iter() {
            rb = rb.header(header, value);
        }

        let req = rb.build().unwrap();

        debug!("executing request: {:?}", req);

        let res = self.client.execute(req);
        debug!("response: {:?}", res);

        debug!("yay");

        Ok(())
    }
}