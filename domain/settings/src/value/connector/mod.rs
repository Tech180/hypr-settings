pub mod constants;
pub mod interfaces;

mod utils;

#[path = "connector.rs"]
mod logic;

#[cfg(test)]
#[path = "connector.test.rs"]
mod tests;

pub use constants::INTERNAL_CONNECTOR_PREFIXES;
pub use interfaces::ConnectorName;
pub use utils::is_identifier;
