#![allow(clippy::disallowed_methods)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use settings_application::network_dto::{PrimaryLink, ScanResult};
use settings_application::{AppError, NetworkApi, NetworkPorts, NetworkService};
use settings_domain::PortError;
use settings_domain::network::{
    AccessPoint, ConnectionProfile, Connectivity, DeviceActionOutcome, DeviceKind, IpConfig,
    IpMethod, IpVersion, MacRandomization, NetworkDevice, NetworkPreferences, QrMatrix,
    SavedWifiProfile, ScanOutcome, ScanReason, ScanSkip, VpnCandidate, VpnConfigKind, VpnDraft,
    VpnManager, VpnMirrorStore, VpnPeerDraft, VpnProfile,
};
use settings_domain::ports::{
    Clock, ConnectionGateway, NetworkEvents, NetworkGateway, NetworkJournal,
    NetworkPreferencesStore, NetworkStats, QrEncoder, Subscription, SystemdUnits, VpnFiles,
    VpnGateway, VpnMirrorStorePort,
};

#[derive(Default)]
struct Inner {
    calls: Vec<String>,
    wifi: Vec<NetworkDevice>,
    wired: Vec<NetworkDevice>,
    aps: Vec<AccessPoint>,
    wifi_enabled: bool,
    rescan: Option<ScanOutcome>,
    connect_fails: bool,
    vpn: Vec<VpnProfile>,
    mirror: VpnMirrorStore,
    counters: (u64, u64),
    offset: Duration,
    qr_text: String,
    profile: Option<ConnectionProfile>,
    saved_wifi: Vec<SavedWifiProfile>,
    import_name: String,
}

struct Fake {
    base: Instant,
    inner: Mutex<Inner>,
}

impl Fake {
    fn new() -> Arc<Self> {
        let fake = Arc::new(Self {
            base: Instant::now(),
            inner: Mutex::new(Inner {
                wifi_enabled: true,
                rescan: Some(ScanOutcome::Ok),
                import_name: "imported".into(),
                ..Inner::default()
            }),
        });
        fake.with(|i| {
            i.wifi = vec![device("wlan0", DeviceKind::Wifi, true, true)];
            i.wired = vec![device("enp3s0", DeviceKind::Ethernet, false, true)];
            i.saved_wifi = vec![SavedWifiProfile {
                id: "Home".into(),
                ssid: "Home".into(),
            }];
        });
        fake
    }

    fn with<T>(&self, f: impl FnOnce(&mut Inner) -> T) -> T {
        f(&mut self.inner.lock().unwrap())
    }

    fn calls(&self) -> Vec<String> {
        self.with(|i| i.calls.clone())
    }

    fn record(&self, call: impl Into<String>) {
        self.with(|i| i.calls.push(call.into()));
    }

    fn advance(&self, by: Duration) {
        self.with(|i| i.offset += by);
    }
}

fn device(iface: &str, kind: DeviceKind, connected: bool, enabled: bool) -> NetworkDevice {
    NetworkDevice {
        iface: iface.into(),
        kind,
        model: String::new(),
        enabled,
        connected,
        speed_mbps: None,
        mtu: None,
        rx_bytes: None,
        tx_bytes: None,
        mac: String::new(),
        ip: String::new(),
        gateway: String::new(),
        dns: vec![],
        ipv6: String::new(),
        connection_name: if connected {
            "Home".into()
        } else {
            String::new()
        },
        internet_access: connected,
    }
}

fn ap(ssid: &str, strength: u32, connected: bool) -> AccessPoint {
    AccessPoint {
        id: format!("{ssid}|bb"),
        ssid: ssid.into(),
        bssid: "bb".into(),
        frequency_mhz: 5180,
        channel: 36,
        strength,
        security: "WPA2".into(),
        connected,
        requires_password: true,
        rate_mbps: 100,
        mode: "Infra".into(),
        device: "wlan0".into(),
    }
}

fn vpn_profile(id: &str, manager: VpnManager, active: bool) -> VpnProfile {
    VpnProfile {
        id: id.into(),
        name: id.into(),
        uuid: format!("uuid-{id}"),
        type_label: "WireGuard".into(),
        active,
        device: String::new(),
        interface_name: "wg0".into(),
        autoconnect: false,
        ipv4_address: "10.0.0.2/32".into(),
        ipv4_method: "manual".into(),
        ipv6_address: String::new(),
        gateway: String::new(),
        dns: vec![],
        mtu: None,
        listen_port: None,
        peer_count: 1,
        peer_public_key: "oldpub".into(),
        state_label: String::new(),
        manager,
        systemd_unit: if manager == VpnManager::Systemd {
            format!("wg-quick-{id}.service")
        } else {
            String::new()
        },
        editable: manager == VpnManager::NetworkManager,
    }
}

impl NetworkGateway for Fake {
    fn devices(&self, kind: DeviceKind) -> Result<Vec<NetworkDevice>, PortError> {
        Ok(self.with(|i| match kind {
            DeviceKind::Wifi => i.wifi.clone(),
            DeviceKind::Ethernet => i.wired.clone(),
        }))
    }

    fn wifi_enabled(&self) -> Result<bool, PortError> {
        Ok(self.with(|i| i.wifi_enabled))
    }

    fn set_wifi_enabled(&self, on: bool) -> Result<(), PortError> {
        self.record(format!("radio {on}"));
        self.with(|i| i.wifi_enabled = on);
        Ok(())
    }

    fn access_points(&self, iface: &str) -> Result<Vec<AccessPoint>, PortError> {
        self.record(format!("list {iface}"));
        Ok(self.with(|i| i.aps.clone()))
    }

    fn rescan(&self, iface: &str) -> ScanOutcome {
        self.record(format!("rescan {iface}"));
        self.with(|i| i.rescan.clone().unwrap())
    }

    fn connect_wifi(
        &self,
        ssid: &str,
        password: Option<&str>,
        iface: Option<&str>,
    ) -> Result<(), PortError> {
        self.record(format!("connect {ssid} {password:?} {iface:?}"));
        if self.with(|i| i.connect_fails) {
            Err(PortError::Unavailable("secrets required".into()))
        } else {
            Ok(())
        }
    }

    fn device_connect(&self, iface: &str) -> DeviceActionOutcome {
        self.record(format!("dev up {iface}"));
        DeviceActionOutcome::Ok
    }

    fn device_disconnect(&self, iface: &str) -> DeviceActionOutcome {
        self.record(format!("dev down {iface}"));
        DeviceActionOutcome::Ok
    }

    fn connectivity(&self) -> Connectivity {
        Connectivity::Full
    }

    fn connection_up(&self, id: &str, iface: Option<&str>) -> Result<(), PortError> {
        self.record(format!("conn up {id} {iface:?}"));
        Ok(())
    }

    fn connection_down(&self, id: &str) -> Result<(), PortError> {
        self.record(format!("conn down {id}"));
        Ok(())
    }
}

impl ConnectionGateway for Fake {
    fn profile(&self, _id: &str) -> Result<Option<ConnectionProfile>, PortError> {
        Ok(self.with(|i| i.profile.clone()))
    }

    fn wifi_secret(&self, _id: &str) -> Result<String, PortError> {
        Ok("pa;ss".into())
    }

    fn set_autoconnect(&self, id: &str, on: bool) -> Result<(), PortError> {
        self.record(format!("autoconnect {id} {on}"));
        Ok(())
    }

    fn set_metered(&self, id: &str, on: bool) -> Result<(), PortError> {
        self.record(format!("metered {id} {on}"));
        Ok(())
    }

    fn set_mac_randomization(&self, id: &str, mode: MacRandomization) -> Result<(), PortError> {
        self.record(format!("mac {id} {mode:?}"));
        Ok(())
    }

    fn apply_ip(&self, id: &str, _v: IpVersion, config: &IpConfig) -> Result<(), PortError> {
        self.record(format!("apply {id} {:?}", config.method));
        Ok(())
    }

    fn forget(&self, id: &str) -> Result<(), PortError> {
        self.record(format!("forget {id}"));
        Ok(())
    }

    fn saved_wifi_profiles(&self) -> Result<Vec<SavedWifiProfile>, PortError> {
        Ok(self.with(|i| i.saved_wifi.clone()))
    }
}

impl VpnGateway for Fake {
    fn profiles(&self) -> Result<Vec<VpnProfile>, PortError> {
        Ok(self.with(|i| i.vpn.clone()))
    }

    fn up(&self, id: &str) -> Result<(), PortError> {
        self.record(format!("vpn up {id}"));
        Ok(())
    }

    fn down(&self, id: &str) -> Result<(), PortError> {
        self.record(format!("vpn down {id}"));
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), PortError> {
        self.record(format!("vpn delete {id}"));
        Ok(())
    }

    fn import_file(&self, kind: VpnConfigKind, path: &str) -> Result<String, PortError> {
        self.record(format!("import {kind:?} {path}"));
        Ok(self.with(|i| i.import_name.clone()))
    }

    fn import_wireguard_text(&self, conf: &str) -> Result<String, PortError> {
        self.record(format!(
            "import text {}",
            conf.contains("PrivateKey = secret")
        ));
        Ok(self.with(|i| i.import_name.clone()))
    }

    fn modify_basics(&self, id: &str, _draft: &VpnDraft) -> Result<(), PortError> {
        self.record(format!("modify {id}"));
        Ok(())
    }

    fn private_key(&self, _id: &str) -> Result<String, PortError> {
        Ok("secret".into())
    }

    fn uuid_of(&self, name: &str) -> Result<String, PortError> {
        Ok(format!("uuid-{name}"))
    }

    fn finalize_import(&self, name: &str, _draft: &VpnDraft) -> Result<(), PortError> {
        self.record(format!("finalize {name}"));
        Ok(())
    }
}

impl SystemdUnits for Fake {
    fn start(&self, unit: &str) -> Result<(), PortError> {
        self.record(format!("unit start {unit}"));
        Ok(())
    }

    fn stop(&self, unit: &str) -> Result<(), PortError> {
        self.record(format!("unit stop {unit}"));
        Ok(())
    }
}

impl NetworkEvents for Fake {
    fn subscribe(&self, _sink: Box<dyn Fn() + Send>) -> Result<Subscription, PortError> {
        Ok(Subscription::new(|| {}))
    }
}

impl NetworkStats for Fake {
    fn counters(&self, _iface: &str) -> Option<(u64, u64)> {
        Some(self.with(|i| i.counters))
    }
}

impl NetworkJournal for Fake {
    fn lines(&self, _limit: usize) -> Result<Vec<String>, PortError> {
        Ok(vec!["journal line".into()])
    }
}

impl QrEncoder for Fake {
    fn encode(&self, text: &str) -> Result<QrMatrix, PortError> {
        self.with(|i| i.qr_text = text.to_owned());
        Ok(QrMatrix {
            size: 1,
            dark: vec![true],
        })
    }
}

impl NetworkPreferencesStore for Fake {
    fn load(&self) -> Result<NetworkPreferences, PortError> {
        Ok(NetworkPreferences::default())
    }

    fn save(&self, _prefs: &NetworkPreferences) -> Result<(), PortError> {
        Ok(())
    }
}

impl VpnMirrorStorePort for Fake {
    fn load(&self) -> Result<VpnMirrorStore, PortError> {
        Ok(self.with(|i| i.mirror.clone()))
    }

    fn save(&self, store: &VpnMirrorStore) -> Result<(), PortError> {
        self.with(|i| i.mirror = store.clone());
        Ok(())
    }
}

impl VpnFiles for Fake {
    fn candidates(&self) -> Vec<VpnCandidate> {
        vec![]
    }

    fn read_text(&self, _path: &str) -> Result<String, PortError> {
        Ok("[Interface]\nPrivateKey = k\nAddress = 10.0.0.2/32\n[Peer]\nPublicKey = pk\nAllowedIPs = 0.0.0.0/0\n".into())
    }
}

impl Clock for Fake {
    fn now(&self) -> Instant {
        self.base + self.with(|i| i.offset)
    }

    fn unix_secs(&self) -> u64 {
        1_700_000_000
    }

    fn sleep(&self, _duration: Duration) {}
}

fn service(fake: &Arc<Fake>) -> NetworkService {
    NetworkService::new(NetworkPorts {
        gateway: fake.clone(),
        connections: fake.clone(),
        vpn: fake.clone(),
        units: fake.clone(),
        events: fake.clone(),
        stats: fake.clone(),
        journal: fake.clone(),
        qr: fake.clone(),
        prefs: fake.clone(),
        mirror: fake.clone(),
        files: fake.clone(),
        clock: fake.clone(),
    })
}

fn rejected(result: Result<impl std::fmt::Debug, AppError>) -> String {
    match result {
        Err(AppError::Rejected(message)) => message,
        other => panic!("expected a rejection, got {other:?}"),
    }
}

#[test]
fn scan_requests_follow_the_cooldown_visibility_and_force_rules() {
    let fake = Fake::new();
    let svc = service(&fake);
    svc.wifi_view().unwrap();

    let scan = |reason, force, visible| svc.request_wifi_scan(reason, force, visible).unwrap();
    assert_eq!(scan(ScanReason::Open, false, true), ScanResult::Scanned);
    assert_eq!(scan(ScanReason::Open, false, true), ScanResult::Listed);
    assert_eq!(scan(ScanReason::Manual, true, true), ScanResult::Scanned);
    assert_eq!(
        scan(ScanReason::Periodic, false, false),
        ScanResult::Skipped(ScanSkip::PageHidden)
    );
    fake.advance(Duration::from_secs(20));
    assert_eq!(scan(ScanReason::Periodic, false, true), ScanResult::Scanned);
    assert_eq!(
        fake.calls()
            .iter()
            .filter(|c| c.starts_with("rescan"))
            .count(),
        3
    );

    let log = svc.network_diagnostics().unwrap();
    assert!(log.actions.iter().any(|e| e.text.contains("cooldown")));
    assert_eq!(log.journal, ["journal line"]);
}

#[test]
fn a_throttled_or_failed_scan_is_reported() {
    let fake = Fake::new();
    let svc = service(&fake);
    svc.wifi_view().unwrap();
    fake.with(|i| i.rescan = Some(ScanOutcome::Rejected("not allowed".into())));
    assert_eq!(
        svc.request_wifi_scan(ScanReason::Manual, true, true)
            .unwrap(),
        ScanResult::Throttled
    );
    fake.with(|i| i.rescan = Some(ScanOutcome::Error("boom".into())));
    assert_eq!(
        rejected(svc.request_wifi_scan(ScanReason::Manual, true, true)),
        "Scan failed."
    );
}

#[test]
fn a_disabled_radio_hides_networks_and_refuses_scans() {
    let fake = Fake::new();
    fake.with(|i| {
        i.aps = vec![ap("Home", 80, true)];
        i.wifi_enabled = false;
    });
    let svc = service(&fake);
    let view = svc.wifi_view().unwrap();
    assert!(!view.enabled && view.access_points.is_empty());
    assert_eq!(
        svc.request_wifi_scan(ScanReason::Manual, true, true)
            .unwrap(),
        ScanResult::Skipped(ScanSkip::WifiDisabled)
    );
}

#[test]
fn the_list_is_sorted_and_survives_a_transient_empty_read() {
    let fake = Fake::new();
    fake.with(|i| i.aps = vec![ap("weak", 20, false), ap("home", 50, true)]);
    let svc = service(&fake);
    let first = svc.wifi_view().unwrap();
    assert_eq!(first.access_points[0].ssid, "home");

    fake.with(|i| i.aps.clear());
    let second = svc.wifi_view().unwrap();
    assert_eq!(second.access_points, first.access_points);
    assert_eq!(second.signature, first.signature);
}

#[test]
fn switching_the_adapter_changes_what_is_listed() {
    let fake = Fake::new();
    fake.with(|i| i.wifi.push(device("wlan1", DeviceKind::Wifi, false, true)));
    let svc = service(&fake);
    assert_eq!(svc.wifi_view().unwrap().selected, "wlan0");
    svc.select_wifi_adapter("wlan1".into()).unwrap();
    assert_eq!(svc.wifi_view().unwrap().selected, "wlan1");
    assert!(fake.calls().contains(&"list wlan1".to_string()));
}

#[test]
fn connecting_passes_the_password_only_when_given() {
    let fake = Fake::new();
    let svc = service(&fake);
    svc.wifi_view().unwrap();
    svc.connect_wifi("Cafe".into(), None).unwrap();
    svc.connect_wifi("Home".into(), Some("hunter2".into()))
        .unwrap();
    svc.connect_wifi("Blank".into(), Some(String::new()))
        .unwrap();
    let calls = fake.calls();
    assert!(calls.contains(&r#"connect Cafe None Some("wlan0")"#.to_string()));
    assert!(calls.contains(&r#"connect Home Some("hunter2") Some("wlan0")"#.to_string()));
    assert!(calls.contains(&r#"connect Blank None Some("wlan0")"#.to_string()));

    fake.with(|i| i.connect_fails = true);
    assert_eq!(
        rejected(svc.connect_wifi("Home".into(), Some("x".into()))),
        "Failed to connect to Home."
    );
}

#[test]
fn disconnect_needs_an_active_connection() {
    let fake = Fake::new();
    let svc = service(&fake);
    svc.wifi_view().unwrap();
    svc.disconnect_wifi().unwrap();
    assert!(fake.calls().contains(&"conn down Home".to_string()));

    fake.with(|i| i.wifi = vec![device("wlan0", DeviceKind::Wifi, false, true)]);
    assert_eq!(
        rejected(svc.disconnect_wifi()),
        "No active Wi-Fi connection."
    );
}

#[test]
fn turning_wifi_on_scans_and_turning_it_off_does_not() {
    let fake = Fake::new();
    let svc = service(&fake);
    svc.wifi_view().unwrap();
    svc.set_wifi_enabled(false).unwrap();
    svc.set_wifi_enabled(true).unwrap();
    let calls = fake.calls();
    assert_eq!(calls.iter().filter(|c| c.starts_with("rescan")).count(), 1);
    assert!(calls.contains(&"radio false".to_string()));
}

#[test]
fn ethernet_without_carrier_is_refused_before_touching_the_device() {
    let fake = Fake::new();
    fake.with(|i| i.wired = vec![device("enp3s0", DeviceKind::Ethernet, false, false)]);
    let svc = service(&fake);
    let view = svc.ethernet_view().unwrap();
    assert_eq!(view.selected, "enp3s0");
    assert!(
        rejected(svc.set_ethernet_enabled(true)).contains("no carrier"),
        "unplugged cable must explain itself"
    );
    assert!(!fake.calls().iter().any(|c| c.starts_with("dev")));

    fake.with(|i| i.wired = vec![device("enp3s0", DeviceKind::Ethernet, true, true)]);
    svc.set_ethernet_enabled(false).unwrap();
    assert!(fake.calls().contains(&"dev down enp3s0".to_string()));
}

#[test]
fn no_ethernet_adapter_is_a_clear_error() {
    let fake = Fake::new();
    fake.with(|i| i.wired.clear());
    let svc = service(&fake);
    svc.ethernet_view().unwrap();
    assert_eq!(
        rejected(svc.set_ethernet_enabled(true)),
        "No Ethernet interface."
    );
}

#[test]
fn invalid_ip_settings_never_reach_networkmanager() {
    let fake = Fake::new();
    let svc = service(&fake);
    let bad = IpConfig {
        method: IpMethod::Manual,
        address: "999.1.1.1".into(),
        prefix_or_mask: "24".into(),
        ..IpConfig::default()
    };
    let message = rejected(svc.apply_ip_settings("Home".into(), IpVersion::V4, bad));
    assert_eq!(message, "Enter a valid IP address.");
    assert!(!fake.calls().iter().any(|c| c.starts_with("apply")));

    svc.apply_ip_settings("Home".into(), IpVersion::V4, IpConfig::default())
        .unwrap();
    assert!(fake.calls().contains(&"apply Home Auto".to_string()));
}

#[test]
fn profile_flags_map_to_nmcli_modes() {
    let fake = Fake::new();
    let svc = service(&fake);
    svc.set_connection_autoconnect("Home".into(), true).unwrap();
    svc.set_connection_metered("Home".into(), true).unwrap();
    svc.set_connection_mac_random("Home".into(), true).unwrap();
    svc.set_connection_mac_random("Home".into(), false).unwrap();
    let calls = fake.calls();
    for expected in [
        "autoconnect Home true",
        "metered Home true",
        "mac Home Random",
        "mac Home Permanent",
    ] {
        assert!(
            calls.contains(&expected.to_string()),
            "{expected}: {calls:?}"
        );
    }
}

#[test]
fn the_qr_code_carries_the_escaped_password() {
    let fake = Fake::new();
    fake.with(|i| {
        i.profile = Some(ConnectionProfile {
            id: "Home".into(),
            name: "Home".into(),
            ssid: "My;Net".into(),
            autoconnect: true,
            metered: false,
            mac_randomization: MacRandomization::Default,
            ipv4: IpConfig::default(),
            ipv6: IpConfig::default(),
            security: "wpa-psk".into(),
            bssid: String::new(),
            frequency_mhz: 0,
            strength: 0,
            standard: "802.11".into(),
        });
    });
    let svc = service(&fake);
    svc.wifi_qr("Home".into()).unwrap();
    assert_eq!(
        fake.with(|i| i.qr_text.clone()),
        r"WIFI:T:WPA;S:My\;Net;P:pa\;ss;H:false;;"
    );

    fake.with(|i| i.profile = None);
    assert_eq!(
        rejected(svc.wifi_qr("Home".into())),
        "No active profile to share."
    );
}

#[test]
fn system_tunnels_go_through_systemd_and_are_read_only() {
    let fake = Fake::new();
    fake.with(|i| {
        i.vpn = vec![
            vpn_profile("mullvad", VpnManager::Systemd, false),
            vpn_profile("work", VpnManager::NetworkManager, true),
        ];
    });
    let svc = service(&fake);
    svc.activate_vpn("mullvad".into()).unwrap();
    svc.deactivate_vpn("work".into()).unwrap();
    let calls = fake.calls();
    assert!(calls.contains(&"unit start wg-quick-mullvad.service".to_string()));
    assert!(calls.contains(&"vpn down work".to_string()));

    assert_eq!(
        rejected(svc.load_vpn_draft("mullvad".into())),
        "System tunnels cannot be edited here."
    );
    assert_eq!(
        rejected(svc.delete_vpn("mullvad".into())),
        "System tunnels cannot be deleted here."
    );
    assert!(!calls.iter().any(|c| c == "vpn delete mullvad"));
}

fn draft(name: &str) -> VpnDraft {
    VpnDraft {
        name: name.into(),
        private_key: "secret".into(),
        addresses: "10.0.0.2/32".into(),
        peers: vec![VpnPeerDraft {
            public_key: "newpub".into(),
            ..VpnPeerDraft::default()
        }],
        ..VpnDraft::default()
    }
}

#[test]
fn creating_a_tunnel_imports_a_conf_and_records_the_mirror() {
    let fake = Fake::new();
    let svc = service(&fake);

    assert_eq!(
        rejected(svc.save_vpn_draft(VpnDraft::default())),
        "Name is required."
    );
    assert!(
        fake.calls().is_empty(),
        "validation runs before any port call"
    );

    svc.save_vpn_draft(draft("office")).unwrap();
    let calls = fake.calls();
    assert!(calls.contains(&"import text true".to_string()), "{calls:?}");
    assert!(calls.contains(&"finalize imported".to_string()));
    let mirror = fake.with(|i| i.mirror.clone());
    let entry = mirror.profiles.get("uuid-office").expect("mirror entry");
    assert_eq!(entry.peers[0].public_key, "newpub");
}

#[test]
fn editing_only_basics_modifies_in_place() {
    let fake = Fake::new();
    fake.with(|i| i.vpn = vec![vpn_profile("work", VpnManager::NetworkManager, true)]);
    let svc = service(&fake);
    let mut form = svc.load_vpn_draft("work".into()).unwrap();
    assert_eq!(form.private_key, "secret");
    assert_eq!(form.peers[0].public_key, "oldpub");

    form.autoconnect = true;
    svc.save_vpn_draft(form).unwrap();
    let calls = fake.calls();
    assert!(calls.contains(&"modify work".to_string()), "{calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("vpn delete")));
}

#[test]
fn changing_peers_replaces_the_tunnel_and_reconnects_an_active_one() {
    let fake = Fake::new();
    fake.with(|i| i.vpn = vec![vpn_profile("work", VpnManager::NetworkManager, true)]);
    let svc = service(&fake);
    let mut form = svc.load_vpn_draft("work".into()).unwrap();
    form.peers[0].public_key = "rotated".into();
    form.name = "work".into();
    form.addresses = "10.0.0.2/32".into();
    fake.with(|i| i.import_name = "work".into());
    svc.save_vpn_draft(form).unwrap();

    let calls = fake.calls();
    let order: Vec<_> = calls
        .iter()
        .filter(|c| {
            c.starts_with("vpn delete") || c.starts_with("import") || c.starts_with("vpn up")
        })
        .cloned()
        .collect();
    assert_eq!(
        order,
        ["vpn delete work", "import text true", "vpn up work"]
    );
}

#[test]
fn deleting_a_tunnel_drops_its_mirror() {
    let fake = Fake::new();
    fake.with(|i| i.vpn = vec![vpn_profile("work", VpnManager::NetworkManager, false)]);
    let svc = service(&fake);
    svc.save_vpn_draft({
        let mut d = draft("work");
        d.id = "work".into();
        d.uuid = "uuid-work".into();
        d.peers[0].public_key = "oldpub".into();
        d
    })
    .unwrap();
    assert!(fake.with(|i| i.mirror.profiles.contains_key("uuid-work")));
    svc.delete_vpn("work".into()).unwrap();
    assert!(fake.with(|i| i.mirror.profiles.is_empty()));
}

#[test]
fn imports_validate_the_path_and_seed_the_mirror() {
    let fake = Fake::new();
    let svc = service(&fake);
    assert!(rejected(svc.import_vpn("wg0.conf".into())).contains("absolute path"));
    assert!(rejected(svc.import_vpn("/tmp/x.txt".into())).contains("Unsupported file"));

    fake.with(|i| i.import_name = "wg-home".into());
    assert_eq!(
        svc.import_vpn("/tmp/wg-home.conf".into()).unwrap(),
        "wg-home"
    );
    assert!(fake.with(|i| i.mirror.profiles.contains_key("uuid-wg-home")));

    svc.import_vpn("/tmp/corp.ovpn".into()).unwrap();
    assert!(
        fake.calls()
            .contains(&"import OpenVpn /tmp/corp.ovpn".to_string())
    );
}

#[test]
fn overview_prefers_the_wired_link_and_counts_tunnels() {
    let fake = Fake::new();
    fake.with(|i| {
        i.vpn = vec![
            vpn_profile("a", VpnManager::NetworkManager, true),
            vpn_profile("b", VpnManager::Systemd, false),
        ];
    });
    let svc = service(&fake);
    let overview = svc.network_overview().unwrap();
    assert_eq!(overview.primary, PrimaryLink::Wifi);
    assert_eq!(overview.title, "Home");
    assert_eq!((overview.vpn_active, overview.vpn_total), (1, 2));
    assert!(overview.has_wifi_profile);

    fake.with(|i| i.wired = vec![device("enp3s0", DeviceKind::Ethernet, true, true)]);
    let overview = svc.network_overview().unwrap();
    assert_eq!(overview.primary, PrimaryLink::Wired);
    assert_eq!(overview.title, "enp3s0");
}

#[test]
fn rates_need_two_samples_and_survive_a_counter_reset() {
    let fake = Fake::new();
    let svc = service(&fake);
    svc.network_overview().unwrap();

    fake.with(|i| i.counters = (1_000, 100));
    assert_eq!(svc.network_rates().rx_per_sec, None);

    fake.advance(Duration::from_secs(2));
    fake.with(|i| i.counters = (5_000, 500));
    let rates = svc.network_rates();
    assert_eq!(rates.rx_per_sec, Some(2_000));
    assert_eq!(rates.tx_per_sec, Some(200));

    fake.advance(Duration::from_secs(1));
    fake.with(|i| i.counters = (10, 10));
    assert_eq!(svc.network_rates().rx_per_sec, Some(0));
}

#[test]
fn the_action_log_can_be_cleared() {
    let fake = Fake::new();
    let svc = service(&fake);
    svc.set_connection_autoconnect("Home".into(), true).unwrap();
    assert!(!svc.network_diagnostics().unwrap().actions.is_empty());
    svc.clear_network_log();
    assert!(svc.network_diagnostics().unwrap().actions.is_empty());
}
