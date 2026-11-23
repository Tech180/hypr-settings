use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TrialError {
    #[error("a trial is already in progress")]
    InProgress,
}
