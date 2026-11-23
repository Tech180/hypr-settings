use serde::{Deserialize, Serialize};

/// Something a brightness change is written to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BrightnessTarget {
    Internal,
    Ddc { bus: u32 },
}

/// What the bar and key bindings need to change brightness without the settings window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrightnessSnapshot {
    pub percent: u8,
    pub sync: bool,
    pub targets: Vec<BrightnessTarget>,
    /// The target of the bar's display, used when sync is off.
    pub host: Option<BrightnessTarget>,
}
