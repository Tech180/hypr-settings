//! Driven ports: everything the core needs from the outside world. All blocking and
//! `Send + Sync`; async is a concern of the driving adapters.

pub mod interfaces;

#[path = "ports.rs"]
mod logic;

pub use interfaces::{
    Backlight, BluetoothEvents, BluetoothExtras, BluetoothGateway, BluetoothPreferencesStore,
    BrightnessState, Clock, ConnectionGateway, Ddc, FeatureFlagStore, HardwareProbe, LiveOutputs,
    NetworkEvents, NetworkGateway, NetworkJournal, NetworkPreferencesStore, NetworkStats,
    NightLight, OutputConfig, PreferencesStore, ProfileController, ProfileStore, QrEncoder,
    Subscription, SystemdUnits, TopologyEvent, TopologyEvents, TopologyReader, TrialJournal,
    VpnFiles, VpnGateway, VpnMirrorStorePort,
};
