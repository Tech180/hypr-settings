pub mod interfaces;

mod utils;

#[path = "mode.rs"]
mod logic;

#[cfg(test)]
#[path = "mode.test.rs"]
mod tests;

pub use interfaces::Mode;
