/// DisplayPort configuration registers: revision, max link rate code and lane count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DpcdCaps {
    pub rev: u8,
    pub max_rate: u8,
    pub max_lanes: u8,
}

/// What the kernel exposes about a connector.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrmProbe {
    pub edid: Option<Vec<u8>>,
    pub vrr_capable: Option<bool>,
    pub max_bpc: Option<u32>,
    pub dpcd: Option<DpcdCaps>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortFamily {
    Edp,
    DisplayPort,
    Hdmi,
    Dvi,
    Other,
}

/// The link a display hangs off, as shown by the port badge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortInfo {
    /// `DisplayPort`, `HDMI`, `eDP`, `DVI` or the connector name.
    pub standard: String,
    /// DPCD revision, HDMI version, `panel`, or `—`.
    pub version: String,
    /// DP and eDP maximum link, for example `HBR3 ×4`; empty when unknown.
    pub link: String,
    /// What the display itself has selected, from DDC/CI.
    pub sink_input: Option<String>,
}
