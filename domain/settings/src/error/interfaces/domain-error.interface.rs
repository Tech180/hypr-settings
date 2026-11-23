use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum DomainError {
    #[error("invalid connector name: {0:?}")]
    InvalidConnector(String),
    #[error("invalid profile name: {0:?}")]
    InvalidProfileName(String),
    #[error("scale {0} is outside 0.5..=4.0")]
    InvalidScale(f64),
    #[error("invalid mode: {0:?}")]
    InvalidMode(String),
}
