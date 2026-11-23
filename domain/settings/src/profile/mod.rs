pub mod constants;
pub mod interfaces;
mod utils;

#[path = "profile.rs"]
mod logic;

#[cfg(test)]
#[path = "profile.test.rs"]
mod tests;

pub use constants::{DEFAULT_PROFILE, HOME_PROFILE};
pub use interfaces::{Criteria, Profile, ProfileName, ProfileOutput, ProfileSet, UpsertOutcome};
pub use utils::{criteria_for, glob_match};
