//! Bluetooth model: device identity, profile rules, and snapshots for the gateway.

mod capabilities;
#[path = "class-from-uuids.rs"]
mod class_from_uuids;
mod device_class;
mod device_id;
#[path = "pick-adapter.rs"]
mod pick_adapter;
mod preferences;
mod profile;
#[path = "profile-rules.rs"]
mod profile_rules;
mod snapshot;

pub use capabilities::BluetoothCapabilities;
pub use class_from_uuids::class_from_uuids;
pub use device_class::DeviceClass;
pub use device_id::DeviceId;
pub use pick_adapter::pick_adapter;
pub use preferences::BluetoothPreferences;
pub use profile::{CodecId, ProfileKind, ProfileState};
pub use profile_rules::{ProfileConflict, apply_profile_toggle};
pub use snapshot::{AdapterSnapshot, CodecSnapshot, DeviceSnapshot, ProfileSnapshot};
