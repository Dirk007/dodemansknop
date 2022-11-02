pub trait Notifier: Send {
    fn notify_failure(&self, id: &String) -> Result<(), &'static str>;
}

#[derive(Copy, Clone)]
pub struct NoOpNotifier {}

impl Notifier for NoOpNotifier {
    fn notify_failure(&self, _id: &String) -> Result<(), &'static str> {
        Ok(())
    }
}
