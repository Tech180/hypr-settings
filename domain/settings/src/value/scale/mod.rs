pub mod constants;
pub mod interfaces;

mod utils;

#[path = "scale.rs"]
mod logic;

#[cfg(test)]
#[path = "scale.test.rs"]
mod tests;

pub use constants::SCALE_PRESETS;
pub use interfaces::Scale;
