use super::ip::IpConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MacRandomization {
    #[default]
    Default,
    Random,
    Stable,
    Permanent,
}

impl MacRandomization {
    pub fn is_random(self) -> bool {
        matches!(self, Self::Random)
    }
}

/// A saved NetworkManager connection (Wi-Fi or wired) with its editable settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub ssid: String,
    pub autoconnect: bool,
    pub metered: bool,
    pub mac_randomization: MacRandomization,
    pub ipv4: IpConfig,
    pub ipv6: IpConfig,
    pub security: String,
    pub bssid: String,
    pub frequency_mhz: u32,
    pub strength: u32,
    pub standard: String,
}

/// How a rescan request ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanOutcome {
    Ok,
    /// NetworkManager throttled it; the cached list is still usable.
    Rejected(String),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceActionOutcome {
    Ok,
    NoCarrier(String),
    Error(String),
}
