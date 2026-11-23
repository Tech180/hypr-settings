//! Read models and results of the Network section.

use settings_domain::network::{
    AccessPoint, ConnectionProfile, Connectivity, LogEntry, NetworkDevice, ScanSkip, VpnProfile,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryLink {
    Wifi,
    Wired,
    Offline,
}

/// What the Network home page shows.
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkOverview {
    pub primary: PrimaryLink,
    /// The network name, the wired interface, or "Not connected".
    pub title: String,
    pub connectivity: Connectivity,
    pub wifi_present: bool,
    pub wifi_enabled: bool,
    /// The SSID Wi-Fi is joined to, if any.
    pub wifi_ssid: Option<String>,
    pub ethernet_present: bool,
    pub ethernet_connected: bool,
    pub vpn_total: usize,
    pub vpn_active: usize,
    /// Whether Properties has an active Wi-Fi profile to open.
    pub has_wifi_profile: bool,
}

/// Bytes per second on the primary link; `None` until two samples exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NetworkRates {
    pub rx_per_sec: Option<u64>,
    pub tx_per_sec: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiSavedEntry {
    pub id: String,
    pub ssid: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WifiView {
    pub enabled: bool,
    pub devices: Vec<NetworkDevice>,
    pub selected: String,
    pub access_points: Vec<AccessPoint>,
    pub saved: Vec<WifiSavedEntry>,
    /// Equal signatures mean the visible list did not change.
    pub signature: String,
    pub scanning: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EthernetView {
    pub devices: Vec<NetworkDevice>,
    pub selected: String,
    pub device: Option<NetworkDevice>,
    /// The saved connection of the selected device, for the IPv4 editor.
    pub profile: Option<ConnectionProfile>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VpnView {
    pub profiles: Vec<VpnProfile>,
}

/// The active Wi-Fi connection.
#[derive(Debug, Clone, PartialEq)]
pub struct WifiProfileView {
    pub profile: Option<ConnectionProfile>,
    pub device: Option<NetworkDevice>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticsView {
    pub actions: Vec<LogEntry>,
    pub journal: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanResult {
    Scanned,
    /// The radio was not asked; the list came from what it already knew.
    Listed,
    /// NetworkManager throttled the scan.
    Throttled,
    Skipped(ScanSkip),
}
