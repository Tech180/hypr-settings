//! Driven adapter for Hyprland: monitor topology, output events and direct output apply.

pub mod events;
pub mod exec;
pub mod live;
pub mod topology;

pub use events::HyprEvents;
pub use live::HyprctlLiveOutputs;
pub use topology::HyprctlTopology;
