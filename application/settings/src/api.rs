use settings_domain::controls::Flag;
use settings_domain::network::{
    ConnectionProfile, IpConfig, IpVersion, NetworkPreferences, QrMatrix, ScanPolicy, ScanReason,
    VpnCandidate, VpnDraft,
};
use settings_domain::ports::{Subscription, TopologyEvent};
use settings_domain::{ConnectorName, LayoutPatch, ProfileName};

use crate::dto::{
    ApplyOutcome, BluetoothView, BrightnessView, DeviceClass, DeviceId, DisplayView,
    NightLightView, OutputControls, ProfileKind, RecoverOutcome, TickOutcome, TrialView,
};
use crate::error::AppError;
use crate::network_dto::{
    DiagnosticsView, EthernetView, NetworkOverview, NetworkRates, ScanResult, VpnView,
    WifiProfileView, WifiView,
};

/// Driving port: what the GTK window and the CLI call.
pub trait DisplayApi: Send + Sync {
    /// Topology, profiles and the running trial, in one snapshot.
    fn view(&self) -> Result<DisplayView, AppError>;
    fn begin_trial(&self, patch: LayoutPatch) -> Result<TrialView, AppError>;
    fn keep_trial(&self) -> Result<(), AppError>;
    fn revert_trial(&self) -> Result<(), AppError>;
    /// Expires and reverts a timed-out trial; cheap when idle.
    fn tick(&self) -> Result<TickOutcome, AppError>;
    fn set_enabled(&self, connector: ConnectorName, enable: bool)
    -> Result<ApplyOutcome, AppError>;
    fn switch_profile(&self, name: ProfileName) -> Result<(), AppError>;
    fn save_snapshot_as(&self, name: ProfileName) -> Result<(), AppError>;
    /// Writes the live layout (and HDR / 10-bit / VRR) into the profile kanshi has selected,
    /// or into `home` when none matches. Returns the profile's name.
    fn save_to_active_profile(&self) -> Result<String, AppError>;
    /// Makes `connector` the display that hosts the bar.
    fn set_bar_host(&self, connector: ConnectorName) -> Result<(), AppError>;
    fn add_custom_hz(&self, connector: ConnectorName, hz: u32) -> Result<(), AppError>;
    fn remove_custom_hz(&self, connector: ConnectorName, hz: u32) -> Result<(), AppError>;
    /// Removes a stored profile; `default` cannot be removed.
    fn delete_profile(&self, name: ProfileName) -> Result<(), AppError>;
    /// Undoes a trial a crashed process left behind.
    fn recover_pending_trial(&self) -> Result<RecoverOutcome, AppError>;
    /// Debounced notifications when outputs may have changed.
    fn subscribe(&self, sink: Box<dyn Fn(TopologyEvent) + Send>) -> Result<Subscription, AppError>;
}

/// Driving port for the hardware side: brightness, DDC/CI controls, HDR / 10-bit / VRR and
/// the night light. Calls here may take a while, so callers run them off the UI thread.
pub trait ControlsApi: Send + Sync {
    fn output_controls(&self, connector: ConnectorName) -> Result<OutputControls, AppError>;
    fn brightness(&self) -> Result<BrightnessView, AppError>;
    /// Looks for displays again, for after a plug or unplug.
    fn refresh_brightness(&self) -> Result<BrightnessView, AppError>;
    /// Follows the sync preference: every display, or only the one hosting the bar.
    fn set_brightness(&self, percent: u8) -> Result<(), AppError>;
    /// A relative change like the brightness keys make; follows the sync preference too and
    /// returns the new value.
    fn adjust_brightness(&self, delta: i32) -> Result<u8, AppError>;
    fn set_sync_brightness(&self, sync: bool) -> Result<(), AppError>;
    fn set_contrast(&self, connector: ConnectorName, percent: u8) -> Result<(), AppError>;
    fn set_overdrive(&self, connector: ConnectorName, percent: u8) -> Result<(), AppError>;
    fn set_input_source(&self, connector: ConnectorName, code: u8) -> Result<(), AppError>;
    fn set_flag(
        &self,
        connector: ConnectorName,
        flag: Flag,
        enable: bool,
    ) -> Result<ApplyOutcome, AppError>;
    /// Re-applies the stored flags of the active profile. Returns how many outputs changed.
    fn restore_flags(&self) -> Result<usize, AppError>;
    fn night_light(&self) -> Result<NightLightView, AppError>;
    fn set_night_light(&self, on: bool) -> Result<(), AppError>;
    fn set_night_kelvin(&self, kelvin: u32) -> Result<(), AppError>;
    /// Re-asserts a manual night light after login.
    fn restore_night_light(&self) -> Result<(), AppError>;
}

/// Driving port for the Bluetooth section.
pub trait BluetoothApi: Send + Sync {
    fn bluetooth_view(&self) -> Result<BluetoothView, AppError>;
    fn set_bluetooth_powered(&self, on: bool) -> Result<(), AppError>;
    fn start_bluetooth_discovery(&self) -> Result<(), AppError>;
    fn stop_bluetooth_discovery(&self) -> Result<(), AppError>;
    fn select_bluetooth_adapter(&self, address: String) -> Result<(), AppError>;
    fn pair_device(&self, id: DeviceId) -> Result<(), AppError>;
    fn connect_device(&self, id: DeviceId) -> Result<(), AppError>;
    fn disconnect_device(&self, id: DeviceId) -> Result<(), AppError>;
    fn forget_device(&self, id: DeviceId) -> Result<(), AppError>;
    fn set_device_class(&self, id: DeviceId, class: DeviceClass) -> Result<(), AppError>;
    fn set_device_profile(
        &self,
        id: DeviceId,
        profile: ProfileKind,
        enabled: bool,
    ) -> Result<(), AppError>;
    fn set_device_codec(&self, id: DeviceId, codec: String) -> Result<(), AppError>;
    fn set_absolute_volume(&self, id: DeviceId, on: bool) -> Result<(), AppError>;
    fn set_pan_enabled(&self, on: bool) -> Result<(), AppError>;
    fn set_avrcp_version(&self, version: String) -> Result<(), AppError>;
    fn set_sample_rate_hz(&self, hz: u32) -> Result<(), AppError>;
    fn set_a2dp_offload_disabled(&self, disabled: bool) -> Result<(), AppError>;
    fn set_hci_snoop_enabled(&self, on: bool) -> Result<(), AppError>;
    fn clear_gatt_cache(&self) -> Result<(), AppError>;
    fn subscribe_bluetooth(&self, sink: Box<dyn Fn() + Send>) -> Result<Subscription, AppError>;
}

/// Driving port for the Network section. Reads are cheap to repeat; commands may take a
/// while (`nmcli`), so callers run them off the UI thread.
pub trait NetworkApi: Send + Sync {
    fn network_overview(&self) -> Result<NetworkOverview, AppError>;
    /// Throughput of the primary link, from counter deltas.
    fn network_rates(&self) -> NetworkRates;
    fn wifi_view(&self) -> Result<WifiView, AppError>;
    fn ethernet_view(&self) -> Result<EthernetView, AppError>;
    fn vpn_view(&self) -> Result<VpnView, AppError>;
    /// The saved connection behind the active Wi-Fi link.
    fn wifi_profile(&self) -> Result<WifiProfileView, AppError>;
    fn connection_profile(&self, id: String) -> Result<Option<ConnectionProfile>, AppError>;
    fn network_diagnostics(&self) -> Result<DiagnosticsView, AppError>;

    fn set_wifi_enabled(&self, on: bool) -> Result<(), AppError>;
    fn select_wifi_adapter(&self, iface: String) -> Result<(), AppError>;
    fn request_wifi_scan(
        &self,
        reason: ScanReason,
        force: bool,
        page_visible: bool,
    ) -> Result<ScanResult, AppError>;
    fn connect_wifi(&self, ssid: String, password: Option<String>) -> Result<(), AppError>;
    fn activate_wifi_connection(&self, id: String) -> Result<(), AppError>;
    fn disconnect_wifi(&self) -> Result<(), AppError>;

    fn set_ethernet_enabled(&self, on: bool) -> Result<(), AppError>;
    fn select_ethernet_adapter(&self, iface: String) -> Result<(), AppError>;

    fn set_connection_autoconnect(&self, id: String, on: bool) -> Result<(), AppError>;
    fn set_connection_metered(&self, id: String, on: bool) -> Result<(), AppError>;
    fn set_connection_mac_random(&self, id: String, random: bool) -> Result<(), AppError>;
    fn apply_ip_settings(
        &self,
        id: String,
        version: IpVersion,
        config: IpConfig,
    ) -> Result<(), AppError>;
    fn forget_connection(&self, id: String) -> Result<(), AppError>;
    fn wifi_qr(&self, id: String) -> Result<QrMatrix, AppError>;

    fn activate_vpn(&self, id: String) -> Result<(), AppError>;
    fn deactivate_vpn(&self, id: String) -> Result<(), AppError>;
    fn load_vpn_draft(&self, id: String) -> Result<VpnDraft, AppError>;
    fn save_vpn_draft(&self, draft: VpnDraft) -> Result<(), AppError>;
    fn delete_vpn(&self, id: String) -> Result<(), AppError>;
    /// Returns the imported connection's name.
    fn import_vpn(&self, path: String) -> Result<String, AppError>;
    fn vpn_candidates(&self) -> Vec<VpnCandidate>;

    fn network_preferences(&self) -> Result<NetworkPreferences, AppError>;
    fn set_scan_policy(&self, policy: ScanPolicy, interval_ms: u32) -> Result<(), AppError>;
    fn clear_network_log(&self);
    fn subscribe_network(&self, sink: Box<dyn Fn() + Send>) -> Result<Subscription, AppError>;
}

/// Everything the windows and the CLI use.
pub trait SettingsApi: DisplayApi + ControlsApi + BluetoothApi + NetworkApi {}

impl<T: DisplayApi + ControlsApi + BluetoothApi + NetworkApi + ?Sized> SettingsApi for T {}
