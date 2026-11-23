pub mod constants;
pub mod interfaces;
mod utils;

#[path = "output.rs"]
mod logic;

#[cfg(test)]
#[path = "output.fixtures.rs"]
pub(crate) mod fixtures;

#[cfg(test)]
#[path = "output.test.rs"]
mod tests;

pub use interfaces::{DisplayKind, Output, OutputIdentity, Topology};
pub use utils::logical_size;
