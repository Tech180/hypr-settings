use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    Hdr,
    TenBit,
    Vrr,
}

/// Compositor flags wanted for one output; a missing key leaves the live value alone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputFlags {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdr: Option<bool>,
    #[serde(rename = "bitdepth10", skip_serializing_if = "Option::is_none")]
    pub ten_bit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vrr: Option<bool>,
}

/// profile name -> connector -> flags
pub type ProfileFlags = BTreeMap<String, BTreeMap<String, OutputFlags>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub supported: bool,
    pub enabled: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureCaps {
    pub hdr: Capability,
    pub ten_bit: Capability,
    pub vrr: Capability,
}

/// What the display and the kernel say about a connector, plus the live compositor state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FeatureProbe {
    /// DRM `vrr_capable`; `None` when sysfs has no such file.
    pub vrr_capable: Option<bool>,
    pub max_bpc: Option<u32>,
    pub edid_hdr: bool,
    pub edid_10bit: bool,
    pub live_hdr: bool,
    pub live_ten_bit: bool,
    pub live_vrr: bool,
}
