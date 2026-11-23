//! Driven adapter for the local system: XDG paths, JSON preferences, the trial journal
//! and the system clock.

mod atomic;
pub mod paths;
pub mod probe;
pub mod stores;

pub use paths::XdgPaths;
pub use probe::SysfsHardwareProbe;
pub use stores::{
    FileTrialJournal, JsonBluetoothPreferencesStore, JsonBrightnessState, JsonFeatureFlagStore,
    JsonNetworkPreferencesStore, JsonPreferencesStore, JsonVpnMirrorStore, SystemClock,
};
