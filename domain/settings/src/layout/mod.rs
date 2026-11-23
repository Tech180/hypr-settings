pub mod interfaces;
mod utils;

#[path = "layout.rs"]
mod logic;

#[cfg(test)]
#[path = "layout.test.rs"]
mod tests;

pub use interfaces::{LayoutPatch, LayoutPolicy, LayoutWarning, ModeRequest, ResolvedPatch};
