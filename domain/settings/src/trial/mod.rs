pub mod interfaces;

#[path = "trial.rs"]
mod logic;

#[cfg(test)]
#[path = "trial.test.rs"]
mod tests;

pub use interfaces::{Trial, TrialKind, TrialPolicy, TrialSlot};
