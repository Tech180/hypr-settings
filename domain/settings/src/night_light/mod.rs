pub mod constants;
pub mod interfaces;

#[path = "night-light.rs"]
mod logic;

#[cfg(test)]
#[path = "night-light.test.rs"]
mod tests;

pub use constants::{KELVIN_DEFAULT, KELVIN_MAX, KELVIN_MIN};
pub use interfaces::Kelvin;
