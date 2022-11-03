use log::debug;
use reqwest::blocking::Client;
use reqwest::Method;
use serde_json::{json, Value};

use crate::notifier::{Notifier, Alert};

#[derive(Clone)]
pub struct WebhookNotifier {
    url: String,
    method: String,
    body: Option<Value>,
    headers: Vec<(String, String)>,

    client: Client,
}

impl WebhookNotifier {
    pub fn new(url: String, method: String, body: Option<Value>, headers: Vec<(String, String)>) -> Self {
        Self {
            url,
            method,
            body,
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

        let mut msg = self.body.clone().unwrap_or(json!({}));

        msg["id"] = json!(alert.id.clone());
        msg["message"] = json!(format!("service {} missed its dead mans switch", alert.id));

        let mut rb = self.client.request(method, &self.url).json(&msg);

        for (header, value) in self.headers.iter() {
            rb = rb.header(header, value);
        }

        let req = rb.build().unwrap();

        debug!("executing request: {:?}", req);

        let res = self.client.execute(req);
        debug!("response: {:?}", res);

        Ok(())
    }
}