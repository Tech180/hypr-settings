use settings_domain::network::{
    ConnectionProfile, IpConfig, IpVersion, NetworkPreferences, QrMatrix, ScanPolicy, ScanReason,
    VpnCandidate, VpnDraft,
};
use settings_domain::ports::Subscription;

use crate::api::NetworkApi;
use crate::error::AppError;
use crate::network_dto::{
    DiagnosticsView, EthernetView, NetworkOverview, NetworkRates, ScanResult, VpnView,
    WifiProfileView, WifiView,
};
use crate::network_ports::NetworkPorts;
use crate::use_cases::network::NetworkCore;

pub struct NetworkService {
    core: NetworkCore,
}

impl NetworkService {
    pub fn new(ports: NetworkPorts) -> Self {
        Self {
            core: NetworkCore::new(ports),
        }
    }
}

impl NetworkApi for NetworkService {
    fn network_overview(&self) -> Result<NetworkOverview, AppError> {
        self.core.network_overview()
    }

    fn network_rates(&self) -> NetworkRates {
        self.core.network_rates()
    }

    fn wifi_view(&self) -> Result<WifiView, AppError> {
        self.core.wifi_view()
    }

    fn ethernet_view(&self) -> Result<EthernetView, AppError> {
        self.core.ethernet_view()
    }

    fn vpn_view(&self) -> Result<VpnView, AppError> {
        self.core.vpn_view()
    }

    fn wifi_profile(&self) -> Result<WifiProfileView, AppError> {
        self.core.wifi_profile()
    }

    fn connection_profile(&self, id: String) -> Result<Option<ConnectionProfile>, AppError> {
        self.core.connection_profile(&id)
    }

    fn network_diagnostics(&self) -> Result<DiagnosticsView, AppError> {
        self.core.network_diagnostics()
    }

    fn set_wifi_enabled(&self, on: bool) -> Result<(), AppError> {
        self.core.set_wifi_enabled(on)
    }

    fn select_wifi_adapter(&self, iface: String) -> Result<(), AppError> {
        self.core.select_wifi_adapter(iface)
    }

    fn request_wifi_scan(
        &self,
        reason: ScanReason,
        force: bool,
        page_visible: bool,
    ) -> Result<ScanResult, AppError> {
        self.core.request_wifi_scan(reason, force, page_visible)
    }

    fn connect_wifi(&self, ssid: String, password: Option<String>) -> Result<(), AppError> {
        self.core.connect_wifi(ssid, password)
    }

    fn activate_wifi_connection(&self, id: String) -> Result<(), AppError> {
        self.core.activate_wifi_connection(&id)
    }

    fn disconnect_wifi(&self) -> Result<(), AppError> {
        self.core.disconnect_wifi()
    }

    fn set_ethernet_enabled(&self, on: bool) -> Result<(), AppError> {
        self.core.set_ethernet_enabled(on)
    }

    fn select_ethernet_adapter(&self, iface: String) -> Result<(), AppError> {
        self.core.select_ethernet_adapter(iface)
    }

    fn set_connection_autoconnect(&self, id: String, on: bool) -> Result<(), AppError> {
        self.core.set_autoconnect(&id, on)
    }

    fn set_connection_metered(&self, id: String, on: bool) -> Result<(), AppError> {
        self.core.set_metered(&id, on)
    }

    fn set_connection_mac_random(&self, id: String, random: bool) -> Result<(), AppError> {
        self.core.set_mac_randomization(&id, random)
    }

    fn apply_ip_settings(
        &self,
        id: String,
        version: IpVersion,
        config: IpConfig,
    ) -> Result<(), AppError> {
        self.core.apply_ip(&id, version, config)
    }

    fn forget_connection(&self, id: String) -> Result<(), AppError> {
        self.core.forget_connection(&id)
    }

    fn wifi_qr(&self, id: String) -> Result<QrMatrix, AppError> {
        self.core.wifi_qr(&id)
    }

    fn activate_vpn(&self, id: String) -> Result<(), AppError> {
        self.core.activate_vpn(&id)
    }

    fn deactivate_vpn(&self, id: String) -> Result<(), AppError> {
        self.core.deactivate_vpn(&id)
    }

    fn load_vpn_draft(&self, id: String) -> Result<VpnDraft, AppError> {
        self.core.load_vpn_draft(&id)
    }

    fn save_vpn_draft(&self, draft: VpnDraft) -> Result<(), AppError> {
        self.core.save_vpn_draft(draft)
    }

    fn delete_vpn(&self, id: String) -> Result<(), AppError> {
        self.core.delete_vpn(&id)
    }

    fn import_vpn(&self, path: String) -> Result<String, AppError> {
        self.core.import_vpn(&path)
    }

    fn vpn_candidates(&self) -> Vec<VpnCandidate> {
        self.core.vpn_candidates()
    }

    fn network_preferences(&self) -> Result<NetworkPreferences, AppError> {
        self.core.scan_preferences()
    }

    fn set_scan_policy(&self, policy: ScanPolicy, interval_ms: u32) -> Result<(), AppError> {
        self.core.set_scan_policy(policy, interval_ms)
    }

    fn clear_network_log(&self) {
        self.core.clear_network_log();
    }

    fn subscribe_network(&self, sink: Box<dyn Fn() + Send>) -> Result<Subscription, AppError> {
        self.core.subscribe(sink)
    }
}
