//! Hardware controls of one output: brightness, contrast, overdrive, input source, the
//! HDR / 10-bit / Adaptive Sync capabilities and the port a display hangs off.

pub mod constants;
pub mod interfaces;
mod utils;

#[cfg(test)]
#[path = "controls.test.rs"]
mod tests;

pub use constants::*;
pub use interfaces::*;
pub use utils::*;
