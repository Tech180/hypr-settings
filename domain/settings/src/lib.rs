//! Pure display-settings model: value objects, layout rules, kanshi profile rules,
//! the trial state machine and the driven ports. No IO lives here.

pub mod arrangement;
pub mod bluetooth;
pub mod controls;
pub mod error;
pub mod layout;
pub mod modes;
pub mod network;
pub mod night_light;
pub mod output;
pub mod ports;
pub mod preferences;
pub mod profile;
pub mod trial;
pub mod value;

pub use bluetooth::{
    AdapterSnapshot, BluetoothCapabilities, BluetoothPreferences, CodecId, CodecSnapshot,
    DeviceClass, DeviceId, DeviceSnapshot, ProfileConflict, ProfileKind, ProfileSnapshot,
    ProfileState, apply_profile_toggle, class_from_uuids,
};
pub use error::{DomainError, LayoutError, PortError, TrialError};
pub use layout::{LayoutPatch, LayoutPolicy, LayoutWarning, ModeRequest, ResolvedPatch};
pub use modes::AdvertisedModes;
pub use night_light::Kelvin;
pub use output::{DisplayKind, Output, OutputIdentity, Topology};
pub use preferences::{DisplayPreferences, TrialRecord};
pub use profile::{
    Criteria, DEFAULT_PROFILE, HOME_PROFILE, Profile, ProfileName, ProfileOutput, ProfileSet,
    UpsertOutcome, criteria_for, glob_match,
};
pub use trial::{Trial, TrialKind, TrialPolicy, TrialSlot};
pub use value::{
    ConnectorName, Mode, Position, RefreshCentiHz, Resolution, SCALE_PRESETS, Scale, Transform,
};
