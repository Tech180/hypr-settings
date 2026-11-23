use std::sync::Arc;

use settings_domain::bluetooth::{DeviceClass, DeviceId, ProfileKind};
use settings_domain::controls::Flag;
use settings_domain::ports::Subscription;
use settings_domain::{ConnectorName, LayoutPatch, ProfileName};

use crate::api::{BluetoothApi, ControlsApi, DisplayApi, NetworkApi};
use crate::bluetooth_service::BluetoothService;
use crate::dto::{
    ApplyOutcome, BluetoothView, BrightnessView, DisplayView, NightLightView, OutputControls,
    RecoverOutcome, TickOutcome, TrialView,
};
use crate::error::AppError;
use crate::network_dto::{
    DiagnosticsView, EthernetView, NetworkOverview, NetworkRates, ScanResult, VpnView,
    WifiProfileView, WifiView,
};
use crate::network_service::NetworkService;
use crate::service::DisplayService;
use settings_domain::network::{
    ConnectionProfile, IpConfig, IpVersion, NetworkPreferences, QrMatrix, ScanPolicy, ScanReason,
    VpnCandidate, VpnDraft,
};

/// Single API surface for the GTK shell and CLI.
pub struct SettingsService {
    display: DisplayService,
    bluetooth: BluetoothService,
    network: NetworkService,
}

impl SettingsService {
    pub fn new(
        display: DisplayService,
        bluetooth: BluetoothService,
        network: NetworkService,
    ) -> Arc<Self> {
        Arc::new(Self {
            display,
            bluetooth,
            network,
        })
    }
}

impl DisplayApi for SettingsService {
    fn view(&self) -> Result<DisplayView, AppError> {
        self.display.view()
    }

    fn begin_trial(&self, patch: LayoutPatch) -> Result<TrialView, AppError> {
        self.display.begin_trial(patch)
    }

    fn keep_trial(&self) -> Result<(), AppError> {
        self.display.keep_trial()
    }

    fn revert_trial(&self) -> Result<(), AppError> {
        self.display.revert_trial()
    }

    fn tick(&self) -> Result<TickOutcome, AppError> {
        self.display.tick()
    }

    fn set_enabled(
        &self,
        connector: ConnectorName,
        enable: bool,
    ) -> Result<ApplyOutcome, AppError> {
        self.display.set_enabled(connector, enable)
    }

    fn switch_profile(&self, name: ProfileName) -> Result<(), AppError> {
        self.display.switch_profile(name)
    }

    fn save_snapshot_as(&self, name: ProfileName) -> Result<(), AppError> {
        self.display.save_snapshot_as(name)
    }

    fn save_to_active_profile(&self) -> Result<String, AppError> {
        self.display.save_to_active_profile()
    }

    fn set_bar_host(&self, connector: ConnectorName) -> Result<(), AppError> {
        self.display.set_bar_host(connector)
    }

    fn add_custom_hz(&self, connector: ConnectorName, hz: u32) -> Result<(), AppError> {
        self.display.add_custom_hz(connector, hz)
    }

    fn remove_custom_hz(&self, connector: ConnectorName, hz: u32) -> Result<(), AppError> {
        self.display.remove_custom_hz(connector, hz)
    }

    fn delete_profile(&self, name: ProfileName) -> Result<(), AppError> {
        self.display.delete_profile(name)
    }

    fn recover_pending_trial(&self) -> Result<RecoverOutcome, AppError> {
        self.display.recover_pending_trial()
    }

    fn subscribe(
        &self,
        sink: Box<dyn Fn(settings_domain::ports::TopologyEvent) + Send>,
    ) -> Result<Subscription, AppError> {
        self.display.subscribe(sink)
    }
}

impl ControlsApi for SettingsService {
    fn output_controls(&self, connector: ConnectorName) -> Result<OutputControls, AppError> {
        self.display.output_controls(connector)
    }

    fn brightness(&self) -> Result<BrightnessView, AppError> {
        self.display.brightness()
    }

    fn refresh_brightness(&self) -> Result<BrightnessView, AppError> {
        self.display.refresh_brightness()
    }

    fn set_brightness(&self, percent: u8) -> Result<(), AppError> {
        self.display.set_brightness(percent)
    }

    fn adjust_brightness(&self, delta: i32) -> Result<u8, AppError> {
        self.display.adjust_brightness(delta)
    }

    fn set_sync_brightness(&self, sync: bool) -> Result<(), AppError> {
        self.display.set_sync_brightness(sync)
    }

    fn set_contrast(&self, connector: ConnectorName, percent: u8) -> Result<(), AppError> {
        self.display.set_contrast(connector, percent)
    }

    fn set_overdrive(&self, connector: ConnectorName, percent: u8) -> Result<(), AppError> {
        self.display.set_overdrive(connector, percent)
    }

    fn set_input_source(&self, connector: ConnectorName, code: u8) -> Result<(), AppError> {
        self.display.set_input_source(connector, code)
    }

    fn set_flag(
        &self,
        connector: ConnectorName,
        flag: Flag,
        enable: bool,
    ) -> Result<ApplyOutcome, AppError> {
        self.display.set_flag(connector, flag, enable)
    }

    fn restore_flags(&self) -> Result<usize, AppError> {
        self.display.restore_flags()
    }

    fn night_light(&self) -> Result<NightLightView, AppError> {
        self.display.night_light()
    }

    fn set_night_light(&self, on: bool) -> Result<(), AppError> {
        self.display.set_night_light(on)
    }

    fn set_night_kelvin(&self, kelvin: u32) -> Result<(), AppError> {
        self.display.set_night_kelvin(kelvin)
    }

    fn restore_night_light(&self) -> Result<(), AppError> {
        self.display.restore_night_light()
    }
}

impl BluetoothApi for SettingsService {
    fn bluetooth_view(&self) -> Result<BluetoothView, AppError> {
        self.bluetooth.bluetooth_view()
    }

    fn set_bluetooth_powered(&self, on: bool) -> Result<(), AppError> {
        self.bluetooth.set_bluetooth_powered(on)
    }

    fn start_bluetooth_discovery(&self) -> Result<(), AppError> {
        self.bluetooth.start_bluetooth_discovery()
    }

    fn stop_bluetooth_discovery(&self) -> Result<(), AppError> {
        self.bluetooth.stop_bluetooth_discovery()
    }

    fn select_bluetooth_adapter(&self, address: String) -> Result<(), AppError> {
        self.bluetooth.select_bluetooth_adapter(address)
    }

    fn pair_device(&self, id: DeviceId) -> Result<(), AppError> {
        self.bluetooth.pair_device(id)
    }

    fn connect_device(&self, id: DeviceId) -> Result<(), AppError> {
        self.bluetooth.connect_device(id)
    }

    fn disconnect_device(&self, id: DeviceId) -> Result<(), AppError> {
        self.bluetooth.disconnect_device(id)
    }

    fn forget_device(&self, id: DeviceId) -> Result<(), AppError> {
        self.bluetooth.forget_device(id)
    }

    fn set_device_class(&self, id: DeviceId, class: DeviceClass) -> Result<(), AppError> {
        self.bluetooth.set_device_class(id, class)
    }

    fn set_device_profile(
        &self,
        id: DeviceId,
        profile: ProfileKind,
        enabled: bool,
    ) -> Result<(), AppError> {
        self.bluetooth.set_device_profile(id, profile, enabled)
    }

    fn set_device_codec(&self, id: DeviceId, codec: String) -> Result<(), AppError> {
        self.bluetooth.set_device_codec(id, codec)
    }

    fn set_absolute_volume(&self, id: DeviceId, on: bool) -> Result<(), AppError> {
        self.bluetooth.set_absolute_volume(id, on)
    }

    fn set_pan_enabled(&self, on: bool) -> Result<(), AppError> {
        self.bluetooth.set_pan_enabled(on)
    }

    fn set_avrcp_version(&self, version: String) -> Result<(), AppError> {
        self.bluetooth.set_avrcp_version(version)
    }

    fn set_sample_rate_hz(&self, hz: u32) -> Result<(), AppError> {
        self.bluetooth.set_sample_rate_hz(hz)
    }

    fn set_a2dp_offload_disabled(&self, disabled: bool) -> Result<(), AppError> {
        self.bluetooth.set_a2dp_offload_disabled(disabled)
    }

    fn set_hci_snoop_enabled(&self, on: bool) -> Result<(), AppError> {
        self.bluetooth.set_hci_snoop_enabled(on)
    }

    fn clear_gatt_cache(&self) -> Result<(), AppError> {
        self.bluetooth.clear_gatt_cache()
    }

    fn subscribe_bluetooth(&self, sink: Box<dyn Fn() + Send>) -> Result<Subscription, AppError> {
        self.bluetooth.subscribe_bluetooth(sink)
    }
}

impl NetworkApi for SettingsService {
    fn network_overview(&self) -> Result<NetworkOverview, AppError> {
        self.network.network_overview()
    }

    fn network_rates(&self) -> NetworkRates {
        self.network.network_rates()
    }

    fn wifi_view(&self) -> Result<WifiView, AppError> {
        self.network.wifi_view()
    }

    fn ethernet_view(&self) -> Result<EthernetView, AppError> {
        self.network.ethernet_view()
    }

    fn vpn_view(&self) -> Result<VpnView, AppError> {
        self.network.vpn_view()
    }

    fn wifi_profile(&self) -> Result<WifiProfileView, AppError> {
        self.network.wifi_profile()
    }

    fn connection_profile(&self, id: String) -> Result<Option<ConnectionProfile>, AppError> {
        self.network.connection_profile(id)
    }

    fn network_diagnostics(&self) -> Result<DiagnosticsView, AppError> {
        self.network.network_diagnostics()
    }

    fn set_wifi_enabled(&self, on: bool) -> Result<(), AppError> {
        self.network.set_wifi_enabled(on)
    }

    fn select_wifi_adapter(&self, iface: String) -> Result<(), AppError> {
        self.network.select_wifi_adapter(iface)
    }

    fn request_wifi_scan(
        &self,
        reason: ScanReason,
        force: bool,
        page_visible: bool,
    ) -> Result<ScanResult, AppError> {
        self.network.request_wifi_scan(reason, force, page_visible)
    }

    fn connect_wifi(&self, ssid: String, password: Option<String>) -> Result<(), AppError> {
        self.network.connect_wifi(ssid, password)
    }

    fn activate_wifi_connection(&self, id: String) -> Result<(), AppError> {
        self.network.activate_wifi_connection(id)
    }

    fn disconnect_wifi(&self) -> Result<(), AppError> {
        self.network.disconnect_wifi()
    }

    fn set_ethernet_enabled(&self, on: bool) -> Result<(), AppError> {
        self.network.set_ethernet_enabled(on)
    }

    fn select_ethernet_adapter(&self, iface: String) -> Result<(), AppError> {
        self.network.select_ethernet_adapter(iface)
    }

    fn set_connection_autoconnect(&self, id: String, on: bool) -> Result<(), AppError> {
        self.network.set_connection_autoconnect(id, on)
    }

    fn set_connection_metered(&self, id: String, on: bool) -> Result<(), AppError> {
        self.network.set_connection_metered(id, on)
    }

    fn set_connection_mac_random(&self, id: String, random: bool) -> Result<(), AppError> {
        self.network.set_connection_mac_random(id, random)
    }

    fn apply_ip_settings(
        &self,
        id: String,
        version: IpVersion,
        config: IpConfig,
    ) -> Result<(), AppError> {
        self.network.apply_ip_settings(id, version, config)
    }

    fn forget_connection(&self, id: String) -> Result<(), AppError> {
        self.network.forget_connection(id)
    }

    fn wifi_qr(&self, id: String) -> Result<QrMatrix, AppError> {
        self.network.wifi_qr(id)
    }

    fn activate_vpn(&self, id: String) -> Result<(), AppError> {
        self.network.activate_vpn(id)
    }

    fn deactivate_vpn(&self, id: String) -> Result<(), AppError> {
        self.network.deactivate_vpn(id)
    }

    fn load_vpn_draft(&self, id: String) -> Result<VpnDraft, AppError> {
        self.network.load_vpn_draft(id)
    }

    fn save_vpn_draft(&self, draft: VpnDraft) -> Result<(), AppError> {
        self.network.save_vpn_draft(draft)
    }

    fn delete_vpn(&self, id: String) -> Result<(), AppError> {
        self.network.delete_vpn(id)
    }

    fn import_vpn(&self, path: String) -> Result<String, AppError> {
        self.network.import_vpn(path)
    }

    fn vpn_candidates(&self) -> Vec<VpnCandidate> {
        self.network.vpn_candidates()
    }

    fn network_preferences(&self) -> Result<NetworkPreferences, AppError> {
        self.network.network_preferences()
    }

    fn set_scan_policy(&self, policy: ScanPolicy, interval_ms: u32) -> Result<(), AppError> {
        self.network.set_scan_policy(policy, interval_ms)
    }

    fn clear_network_log(&self) {
        self.network.clear_network_log()
    }

    fn subscribe_network(&self, sink: Box<dyn Fn() + Send>) -> Result<Subscription, AppError> {
        self.network.subscribe_network(sink)
    }
}
