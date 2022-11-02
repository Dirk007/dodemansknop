use config::{Config, File, ConfigError, Environment};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub notifier_type: String,
    pub webhook: Option<WebhookSettings>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookSettings {
    pub url: String,

    #[serde(default = "default_method")]
    pub method: String,

    pub headers: Option<Vec<(String, String)>>,
}

fn default_method() -> String {
    "POST".to_string()
}

pub fn retrieve_settings(file: Option<&str>) -> Result<Settings, ConfigError> {
    let mut b = Config::builder();

    if file.is_some() {
        b = b.add_source(File::with_name(file.unwrap()));
    }

    b = b.add_source(Environment::with_prefix("DODEMANSKNOP").separator("_"));
    b.build()?.try_deserialize()
}