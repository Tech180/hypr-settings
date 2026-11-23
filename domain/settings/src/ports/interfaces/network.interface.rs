use super::subscription::Subscription;
use crate::error::PortError;
use crate::network::{
    AccessPoint, ConnectionProfile, Connectivity, DeviceActionOutcome, DeviceKind, IpConfig,
    IpVersion, MacRandomization, NetworkDevice, NetworkPreferences, QrMatrix, ScanOutcome,
    VpnCandidate, VpnConfigKind, VpnDraft, VpnMirrorStore, VpnProfile,
};

/// Wi-Fi radio and wired devices, as NetworkManager sees them.
pub trait NetworkGateway: Send + Sync {
    fn devices(&self, kind: DeviceKind) -> Result<Vec<NetworkDevice>, PortError>;
    /// Whether the Wi-Fi radio is switched on, system-wide.
    fn wifi_enabled(&self) -> Result<bool, PortError>;
    fn set_wifi_enabled(&self, on: bool) -> Result<(), PortError>;
    /// Raw rows for `iface`; the application normalizes them.
    fn access_points(&self, iface: &str) -> Result<Vec<AccessPoint>, PortError>;
    fn rescan(&self, iface: &str) -> ScanOutcome;
    fn connect_wifi(
        &self,
        ssid: &str,
        password: Option<&str>,
        iface: Option<&str>,
    ) -> Result<(), PortError>;
    fn device_connect(&self, iface: &str) -> DeviceActionOutcome;
    fn device_disconnect(&self, iface: &str) -> DeviceActionOutcome;
    fn connectivity(&self) -> Connectivity;
    fn connection_up(&self, id: &str, iface: Option<&str>) -> Result<(), PortError>;
    fn connection_down(&self, id: &str) -> Result<(), PortError>;
}

/// Saved connection settings.
pub trait ConnectionGateway: Send + Sync {
    fn profile(&self, id: &str) -> Result<Option<ConnectionProfile>, PortError>;
    /// The Wi-Fi password; empty for an open network.
    fn wifi_secret(&self, id: &str) -> Result<String, PortError>;
    fn set_autoconnect(&self, id: &str, on: bool) -> Result<(), PortError>;
    fn set_metered(&self, id: &str, on: bool) -> Result<(), PortError>;
    fn set_mac_randomization(&self, id: &str, mode: MacRandomization) -> Result<(), PortError>;
    /// Writes the settings and re-activates the connection.
    fn apply_ip(&self, id: &str, version: IpVersion, config: &IpConfig) -> Result<(), PortError>;
    fn forget(&self, id: &str) -> Result<(), PortError>;
    fn saved_wifi_profiles(&self) -> Result<Vec<crate::network::SavedWifiProfile>, PortError>;
}

pub trait VpnGateway: Send + Sync {
    /// NetworkManager tunnels and system `wg-quick` tunnels, active first.
    fn profiles(&self) -> Result<Vec<VpnProfile>, PortError>;
    fn up(&self, id: &str) -> Result<(), PortError>;
    fn down(&self, id: &str) -> Result<(), PortError>;
    fn delete(&self, id: &str) -> Result<(), PortError>;
    /// Returns the imported connection's name.
    fn import_file(&self, kind: VpnConfigKind, path: &str) -> Result<String, PortError>;
    /// Imports a WireGuard conf held in memory (kept off disk by the adapter beyond a
    /// private temp file).
    fn import_wireguard_text(&self, conf: &str) -> Result<String, PortError>;
    /// Changes everything except the peers, in place.
    fn modify_basics(&self, id: &str, draft: &VpnDraft) -> Result<(), PortError>;
    fn private_key(&self, id: &str) -> Result<String, PortError>;
    fn uuid_of(&self, name: &str) -> Result<String, PortError>;
    /// Renames an imported connection and applies the draft's basics.
    fn finalize_import(&self, imported_name: &str, draft: &VpnDraft) -> Result<(), PortError>;
}

/// Starts and stops system units; polkit decides whether the user may.
pub trait SystemdUnits: Send + Sync {
    fn start(&self, unit: &str) -> Result<(), PortError>;
    fn stop(&self, unit: &str) -> Result<(), PortError>;
}

pub trait NetworkEvents: Send + Sync {
    /// Calls `sink` (debounced by the adapter) when a device, connection or the
    /// connectivity may have changed.
    fn subscribe(&self, sink: Box<dyn Fn() + Send>) -> Result<Subscription, PortError>;
}

pub trait NetworkStats: Send + Sync {
    /// Total received and transmitted bytes of an interface.
    fn counters(&self, iface: &str) -> Option<(u64, u64)>;
}

pub trait NetworkJournal: Send + Sync {
    fn lines(&self, limit: usize) -> Result<Vec<String>, PortError>;
}

pub trait QrEncoder: Send + Sync {
    fn encode(&self, text: &str) -> Result<QrMatrix, PortError>;
}

pub trait NetworkPreferencesStore: Send + Sync {
    fn load(&self) -> Result<NetworkPreferences, PortError>;
    fn save(&self, prefs: &NetworkPreferences) -> Result<(), PortError>;
}

pub trait VpnMirrorStorePort: Send + Sync {
    fn load(&self) -> Result<VpnMirrorStore, PortError>;
    fn save(&self, store: &VpnMirrorStore) -> Result<(), PortError>;
}

pub trait VpnFiles: Send + Sync {
    /// Config files near the user, newest first.
    fn candidates(&self) -> Vec<VpnCandidate>;
    fn read_text(&self, path: &str) -> Result<String, PortError>;
}
