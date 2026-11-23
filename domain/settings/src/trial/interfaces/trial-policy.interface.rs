use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialPolicy {
    pub timeout: Duration,
}
