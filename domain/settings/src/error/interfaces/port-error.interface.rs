use thiserror::Error;

/// Failure reported by a driven adapter. Adapter-specific error types never leak inward.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PortError {
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("invalid data: {0}")]
    Invalid(String),
    #[error("io: {0}")]
    Io(String),
}
