use crate::notifier::Notifier;

#[derive(Clone)]
pub struct WebhookNotifier {
    url: String,
    method: String,
    headers: Vec<(String, String)>,
}

impl WebhookNotifier {
    pub fn new(url: String, method: String, headers: Vec<(String, String)>) -> Self {
        Self {
            url,
            method,
            headers,
        }
    }
}

impl Notifier for WebhookNotifier {
    fn notify_failure(&self, id: &String) -> Result<(), &'static str> {
        Ok(())
    }
}