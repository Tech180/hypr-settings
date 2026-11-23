pub mod interfaces;

#[path = "display-preferences.rs"]
mod logic;

#[cfg(test)]
#[path = "display-preferences.test.rs"]
mod tests;

pub use interfaces::{DisplayPreferences, TrialRecord};
