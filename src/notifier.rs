#[derive(Clone)]
pub struct Alert {
    pub id: String,
}

pub trait Notifier: Send {
    fn notify_failure(&self, alert: Alert) -> Result<(), &'static str>;
}

#[derive(Copy, Clone)]
pub struct NoOpNotifier {}

impl Notifier for NoOpNotifier {
    fn notify_failure(&self, _alert: Alert) -> Result<(), &'static str> {
        Ok(())
    }
}
