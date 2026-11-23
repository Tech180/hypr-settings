//! Plain data handed to driving adapters. Chips carry primitives so UI code can
//! render them without touching domain rules.

use settings_domain::controls::{Control, FeatureCaps, InputSources, PortInfo};
use settings_domain::{DisplayKind, TrialKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// The compositor reports the requested state.
    Verified,
    /// Applied, but the compositor did not confirm in time.
    Unverified,
    /// Kanshi was unavailable; the change went to the compositor directly.
    Degraded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolutionChip {
    pub width: u32,
    pub height: u32,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HzChip {
    pub hz: u32,
    pub active: bool,
    /// A rate the user pinned; it can be removed again.
    pub custom: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScaleChip {
    pub hundredths: u16,
    pub label: String,
    pub active: bool,
}

/// One rotation. `hypr` already carries the output's current mirror bit, so
/// choosing a chip never undoes a flip.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformChip {
    pub hypr: i32,
    pub label: &'static str,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputView {
    pub connector: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub kind: DisplayKind,
    pub enabled: bool,
    pub focused: bool,
    pub ultrawide: bool,
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub x: i32,
    pub y: i32,
    /// Size in the compositor's layout space (scaled, rotated). Zero without a mode.
    pub logical_width: i32,
    pub logical_height: i32,
    /// Mirrored (Hyprland transforms 4-7).
    pub flipped: bool,
    /// Where the output goes if it is enabled now; meaningful while disabled.
    pub suggested_x: i32,
    pub suggested_y: i32,
    pub resolutions: Vec<ResolutionChip>,
    pub hz_options: Vec<HzChip>,
    pub scale_options: Vec<ScaleChip>,
    pub transform_options: Vec<TransformChip>,
}

/// One stored output of a profile, ready to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileOutputView {
    /// Display name: the description or connector, `Any display`, or `Other displays`.
    pub label: String,
    pub enabled: Option<bool>,
    pub position: Option<(i32, i32)>,
    /// `1920x1080@60 · 1.5×` style summary of mode and scale.
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileView {
    pub name: String,
    pub output_count: usize,
    pub active: bool,
    /// `default` is the bootstrap fallback and cannot be removed.
    pub removable: bool,
    pub outputs: Vec<ProfileOutputView>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrialView {
    pub connector: String,
    pub kind: TrialKind,
    pub remaining_secs: u64,
    pub timeout_secs: u64,
    pub outcome: ApplyOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayView {
    pub outputs: Vec<OutputView>,
    pub profiles: Vec<ProfileView>,
    pub active_profile: Option<String>,
    /// The display that hosts the bar.
    pub bar_host: Option<String>,
    pub trial: Option<TrialView>,
    /// Layout problems worth showing, such as displays the pointer cannot reach.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TickOutcome {
    Idle,
    Active(TrialView),
    /// The trial timed out and the previous layout was restored.
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoverOutcome {
    Nothing,
    Restored { connector: String },
}

/// Hardware controls of one output; slow to build because it talks to the display.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputControls {
    pub connector: String,
    pub brightness: Control,
    pub contrast: Control,
    pub overdrive: Control,
    pub input: InputSources,
    pub caps: FeatureCaps,
    pub port: Option<PortInfo>,
}

/// The shared brightness slider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrightnessView {
    pub percent: Option<u8>,
    pub sync: bool,
    /// How many displays a synchronised change reaches.
    pub targets: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NightLightView {
    pub on: bool,
    pub kelvin: u32,
}

// --- Bluetooth ---

pub use settings_domain::{BluetoothCapabilities, DeviceClass, DeviceId, ProfileKind};

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct BluetoothView {
    pub powered: bool,
    pub alias: String,
    pub discovering: bool,
    pub adapters: Vec<AdapterView>,
    pub adapter: Option<AdapterView>,
    /// Saved adapter MAC that is no longer present; the active adapter is a fallback.
    pub missing_saved_adapter: Option<String>,
    pub paired: Vec<DeviceView>,
    pub discovered: Vec<DeviceView>,
    pub capabilities: BluetoothCapabilities,
    pub advanced: AdvancedView,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AdapterView {
    pub name: String,
    pub address: String,
    pub alias: String,
    pub powered: bool,
    pub lmp_version: Option<String>,
    pub modalias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DeviceView {
    pub id: DeviceId,
    pub name: String,
    pub connected: bool,
    pub paired: bool,
    pub battery_percent: Option<u8>,
    pub class: DeviceClass,
    pub profiles: DeviceProfileView,
    pub codec: Option<CodecView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct DeviceProfileView {
    pub media_audio: ProfileToggleView,
    pub phone_calls: ProfileToggleView,
    pub contact_sharing: ProfileToggleView,
    pub absolute_volume: ProfileToggleView,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct ProfileToggleView {
    pub available: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CodecView {
    pub active: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct AdvancedView {
    pub pan_enabled: bool,
    pub avrcp_version: Option<String>,
    pub avrcp_options: Vec<String>,
    pub sample_rate_hz: Option<u32>,
    pub sample_rate_options: Vec<u32>,
    pub a2dp_offload_disabled: bool,
    pub hci_snoop: bool,
    pub auracast_available: bool,
}
