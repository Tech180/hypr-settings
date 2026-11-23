use settings_domain::{LayoutError, PortError, TrialError};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum AppError {
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error(transparent)]
    Trial(#[from] TrialError),
    #[error(transparent)]
    Port(#[from] PortError),
    #[error("{0}")]
    Rejected(String),
}
