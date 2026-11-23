pub mod interfaces;

mod utils;

#[path = "resolution.rs"]
mod logic;

pub use interfaces::Resolution;
