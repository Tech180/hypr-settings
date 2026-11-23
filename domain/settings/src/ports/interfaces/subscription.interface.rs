/// Cancels its source when dropped.
pub struct Subscription(pub(crate) Option<Box<dyn FnOnce() + Send>>);
