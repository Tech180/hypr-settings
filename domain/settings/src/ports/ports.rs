use crate::ports::interfaces::Subscription;

impl Subscription {
    pub fn new(cancel: impl FnOnce() + Send + 'static) -> Self {
        Self(Some(Box::new(cancel)))
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(cancel) = self.0.take() {
            cancel();
        }
    }
}
