pub mod constants;
pub mod interfaces;

#[path = "arrangement.rs"]
mod logic;

#[cfg(test)]
#[path = "arrangement.test.rs"]
mod tests;

pub use interfaces::{Layout, Rect, Tile};
