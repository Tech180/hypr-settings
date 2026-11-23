pub mod interfaces;

#[path = "position.rs"]
mod logic;

mod utils;

#[cfg(test)]
#[path = "position.test.rs"]
mod tests;

pub use interfaces::Position;
