use anyhow::Result;
use log::debug;
use reqwest::{blocking::Client, Method};
use serde::Serialize;
use serde_json::json;

use crate::notifier::{Alert, Notifier};

#[derive(Clone)]
pub struct SlackNotifier {
    url: String,
    icon_emoji: String,
    color: String,

    client: Client,
}

#[derive(Serialize)]
struct SlackWebhookBody {
    text: String,
}

impl SlackNotifier {
    pub fn new(url: String, icon_emoji: String, color: String) -> Self {
        Self {
            url,
            icon_emoji,
            color,
            client: Client::new(),
        }
    }
}

impl Notifier for SlackNotifier {
    fn notify_failure(&self, alert: Alert) -> Result<()> {
        let body = json!({
            "attachments": [{
                "color": self.color,
                "blocks": [
                    {
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": format!("**{} Dead Mans Switch missed**\nService {} missed its dead mans switch", self.icon_emoji, alert.id),
                            "emoji": true
                        }
                    }
                ]
            }]
        });

        let req = self.client.request(Method::POST, &self.url).json(&body).build()?;

        debug!("executing request: {:?}", req);

        let res = self.client.execute(req);
        debug!("response: {:?}", res);

        Ok(())
    }
}
