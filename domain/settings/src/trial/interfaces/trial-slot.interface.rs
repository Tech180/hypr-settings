use super::trial::Trial;

/// Holds at most one trial.
#[derive(Debug, Default)]
pub struct TrialSlot(pub(crate) Option<Trial>);
