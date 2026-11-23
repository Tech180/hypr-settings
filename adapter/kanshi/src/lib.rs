//! Driven adapter for kanshi: conf text parsing, the atomic runtime conf store and
//! the `kanshictl` controller.

pub mod controller;
pub mod exec;
pub mod format;
pub mod store;

pub use controller::Kanshictl;
pub use store::KanshiConfFile;
