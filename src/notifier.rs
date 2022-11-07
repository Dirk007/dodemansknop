use log::info;

#[derive(Clone, Debug)]
pub struct Alert {
    pub id: String,
}

pub trait Notifier: Send {
    fn notify_failure(&self, alert: Alert) -> Result<(), &'static str>;
}

#[derive(Copy, Clone)]
pub struct NoOpNotifier {}

impl Notifier for NoOpNotifier {
    fn notify_failure(&self, alert: Alert) -> Result<(), &'static str> {
        info!("missed alert for {}: {:?}", alert.id, alert);
        Ok(())
    }
}
