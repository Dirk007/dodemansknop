use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub notifiers: Vec<NotifierSettings>,
    pub timeout: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NotifierSettings {
    #[serde(rename = "type")]
    pub notifier_type: String,
    pub webhook: Option<WebhookSettings>,
    pub slack: Option<SlackSettings>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SlackSettings {
    pub url: String,
    pub icon_emoji: String,
    pub color: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebhookSettings {
    pub url: String,

    #[serde(default = "default_method")]
    pub method: String,

    pub body: Option<Value>,
    pub headers: Option<Vec<(String, String)>>,
}

fn default_method() -> String {
    "POST".to_string()
}

pub fn retrieve_settings(file: Option<String>) -> Result<Settings, ConfigError> {
    let mut b = Config::builder();

    if file.is_some() {
        b = b.add_source(File::with_name(file.unwrap().as_str()));
    }

    b = b.add_source(Environment::with_prefix("DODEMANSKNOP").separator("_"));
    b.build()?.try_deserialize()
}
