pub mod interfaces;

#[path = "modes.rs"]
mod logic;

#[cfg(test)]
#[path = "modes.test.rs"]
mod tests;

pub use interfaces::AdvertisedModes;
