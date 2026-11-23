use crate::error::DomainError;

use super::constants::INTERNAL_CONNECTOR_PREFIXES;
use super::interfaces::ConnectorName;
use super::utils::is_identifier;

impl ConnectorName {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_internal(&self) -> bool {
        INTERNAL_CONNECTOR_PREFIXES
            .iter()
            .any(|p| self.0.starts_with(p))
    }
}

impl TryFrom<String> for ConnectorName {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if is_identifier(&value) {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidConnector(value))
        }
    }
}

impl TryFrom<&str> for ConnectorName {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_owned())
    }
}

impl From<ConnectorName> for String {
    fn from(value: ConnectorName) -> Self {
        value.0
    }
}
