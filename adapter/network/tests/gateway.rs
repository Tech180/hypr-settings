use std::sync::{Arc, Mutex};

use adapter_network::{Exec, ExecError, Nm, NmGateway, NmVpn, Systemctl};
use settings_domain::network::{
    DeviceActionOutcome, DeviceKind, IpConfig, IpMethod, IpVersion, ScanOutcome, VpnDraft,
    VpnManager,
};
use settings_domain::ports::{ConnectionGateway, NetworkGateway, VpnGateway};

/// Answers by the full command line and remembers every call.
#[derive(Default)]
struct Script {
    answers: Mutex<Vec<(String, Result<String, String>)>>,
    calls: Mutex<Vec<String>>,
}

impl Script {
    fn on(&self, command: &str, answer: Result<&str, &str>) -> &Self {
        self.answers.lock().unwrap().push((
            command.to_owned(),
            answer.map(str::to_owned).map_err(str::to_owned),
        ));
        self
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl Exec for Script {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError> {
        let line = format!("{program} {}", args.join(" "));
        self.calls.lock().unwrap().push(line.clone());
        self.answers
            .lock()
            .unwrap()
            .iter()
            .find(|(command, _)| *command == line)
            .map_or_else(
                || Err(ExecError(format!("unscripted: {line}"))),
                |(_, answer)| answer.clone().map_err(ExecError),
            )
    }
}

fn gateway(script: &Arc<Script>) -> NmGateway {
    NmGateway::new(Nm::new(script.clone()))
}

const STATUS: &str = "enp3s0:ethernet:connected:Wired connection 1\nwlan0:wifi:connected:Home\n";

#[test]
fn devices_are_listed_by_kind_with_their_details() {
    let script = Arc::new(Script::default());
    script
        .on(
            "nmcli -t -f DEVICE,TYPE,STATE,CONNECTION device status",
            Ok(STATUS),
        )
        .on(
            "nmcli -t -f all device show enp3s0",
            Ok(include_str!("fixtures/device-show-eth.txt")),
        );
    let devices = gateway(&script).devices(DeviceKind::Ethernet).unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].iface, "enp3s0");
    assert_eq!(devices[0].speed_mbps, Some(1000));
    assert!(!script.calls().iter().any(|c| c.contains("wlan0")));
}

#[test]
fn a_failing_status_is_an_error_but_a_vanished_device_is_skipped() {
    let script = Arc::new(Script::default());
    script.on(
        "nmcli -t -f DEVICE,TYPE,STATE,CONNECTION device status",
        Ok(STATUS),
    );
    // No answer for `device show`: the device went away between the two calls.
    assert!(
        gateway(&script)
            .devices(DeviceKind::Ethernet)
            .unwrap()
            .is_empty()
    );

    let script = Arc::new(Script::default());
    assert!(gateway(&script).devices(DeviceKind::Wifi).is_err());
}

#[test]
fn wifi_listing_scans_and_connects_with_argument_arrays() {
    let script = Arc::new(Script::default());
    script
        .on(
            "nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY,BSSID,FREQ,CHAN,RATE,MODE,DEVICE device wifi list ifname wlan0",
            Ok(include_str!("fixtures/wifi-list.txt")),
        )
        .on("nmcli device wifi rescan ifname wlan0", Err("Error: Scanning not allowed immediately following previous scan."))
        .on("nmcli device wifi connect My SSID password pa ss ifname wlan0", Ok(""))
        .on("nmcli -t -f WIFI radio", Ok("enabled\n"));
    let gw = gateway(&script);

    assert_eq!(gw.access_points("wlan0").unwrap().len(), 4);
    assert!(matches!(gw.rescan("wlan0"), ScanOutcome::Rejected(_)));
    gw.connect_wifi("My SSID", Some("pa ss"), Some("wlan0"))
        .unwrap();
    assert!(gw.wifi_enabled().unwrap());

    // The ssid and password are single arguments, never split or shell-parsed.
    let calls = script.calls();
    assert!(
        calls
            .contains(&"nmcli device wifi connect My SSID password pa ss ifname wlan0".to_string())
    );
}

#[test]
fn an_unplugged_cable_is_reported_as_no_carrier() {
    let script = Arc::new(Script::default());
    script.on(
        "nmcli device connect enp3s0",
        Err("Error: Connection activation failed: No carrier"),
    );
    assert!(matches!(
        gateway(&script).device_connect("enp3s0"),
        DeviceActionOutcome::NoCarrier(_)
    ));
}

#[test]
fn ip_changes_modify_then_reactivate() {
    let script = Arc::new(Script::default());
    script
        .on(
            "nmcli connection modify id Home ipv4.method manual ipv4.addresses 10.0.0.5/24 ipv4.gateway 10.0.0.1 ipv4.dns 1.1.1.1",
            Ok(""),
        )
        .on("nmcli connection up id Home", Ok(""));
    gateway(&script)
        .apply_ip(
            "Home",
            IpVersion::V4,
            &IpConfig {
                method: IpMethod::Manual,
                address: "10.0.0.5".into(),
                prefix_or_mask: "24".into(),
                gateway: "10.0.0.1".into(),
                dns: vec!["1.1.1.1".into()],
            },
        )
        .unwrap();
    assert_eq!(script.calls().len(), 2);
}

#[test]
fn a_secret_is_read_only_with_show_secrets() {
    let script = Arc::new(Script::default());
    script.on(
        "nmcli --show-secrets -t connection show id Home",
        Ok("802-11-wireless-security.psk:hunter2\n"),
    );
    assert_eq!(gateway(&script).wifi_secret("Home").unwrap(), "hunter2");
}

fn vpn(script: &Arc<Script>) -> NmVpn {
    NmVpn::new(
        Nm::new(script.clone()),
        Systemctl::new(script.clone()),
        std::env::temp_dir().join(format!("hs-vpn-{}", std::process::id())),
    )
}

#[test]
fn nm_tunnels_and_wg_quick_units_are_merged_active_first() {
    let script = Arc::new(Script::default());
    script
        .on(
            "nmcli -t -f NAME,UUID,TYPE,DEVICE,FILENAME connection show",
            Ok("Home:uuid-1:802-11-wireless:wlan0:/etc/x\noffice:uuid-2:wireguard::/etc/NetworkManager/system-connections/office.nmconnection\n"),
        )
        .on(
            "nmcli -t -f DEVICE,TYPE,STATE,CONNECTION device status",
            Ok("wlan0:wifi:connected:Home\nmullvad:wireguard:connected (externally):\n"),
        )
        .on(
            "nmcli -t connection show id office",
            Ok("connection.id:office\nconnection.uuid:uuid-2\nconnection.type:wireguard\nconnection.autoconnect:no\nconnection.interface-name:wg-office\nipv4.addresses:10.0.0.2/32\nipv4.method:manual\nwireguard.peers:--\nwireguard.listen-port:51820\n"),
        )
        .on("nmcli -t -f all device show mullvad", Ok("GENERAL.STATE:100 (connected)\nIP4.ADDRESS[1]:10.64.0.2/32\nGENERAL.MTU:1380\n"))
        .on(
            "systemctl list-unit-files wg-quick-*.service --no-legend --plain",
            Ok("wg-quick-mullvad.service enabled enabled\nwg-quick-lab.service enabled enabled\n"),
        )
        .on("systemctl show wg-quick-mullvad.service -p LoadState --value", Ok("loaded\n"))
        .on("systemctl show wg-quick-mullvad.service -p UnitFileState --value", Ok("enabled\n"))
        .on("systemctl is-active wg-quick-mullvad.service", Ok("active\n"))
        .on("systemctl is-active wg-quick-lab.service", Err("inactive"));

    let profiles = vpn(&script).profiles().unwrap();
    let names: Vec<_> = profiles
        .iter()
        .map(|p| (p.name.as_str(), p.active))
        .collect();
    assert_eq!(
        names,
        [("mullvad", true), ("lab", false), ("office", false)]
    );

    let mullvad = &profiles[0];
    assert_eq!(mullvad.manager, VpnManager::Systemd);
    assert_eq!(mullvad.systemd_unit, "wg-quick-mullvad.service");
    assert!(!mullvad.editable);
    assert_eq!(mullvad.ipv4_address, "10.64.0.2/32");
    assert_eq!(mullvad.mtu, Some(1380));

    let lab = &profiles[1];
    assert_eq!(lab.state_label, "Stopped");

    let office = &profiles[2];
    assert_eq!(office.manager, VpnManager::NetworkManager);
    assert!(office.editable);
    assert_eq!(office.listen_port, Some(51820));
    assert_eq!(office.interface_name, "wg-office");
    assert_eq!(office.peer_count, 0);
}

#[test]
fn a_wireguard_conf_is_imported_from_a_private_temp_file() {
    use std::os::unix::fs::PermissionsExt;

    let script = Arc::new(Script::default());
    // The path is only known at run time, so answer every import the same way.
    struct Capture(Arc<Script>, Mutex<Option<(String, u32)>>);
    impl Exec for Capture {
        fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError> {
            if args.starts_with(&["connection", "import", "type", "wireguard", "file"]) {
                let path = args[5];
                let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
                let text = std::fs::read_to_string(path).unwrap();
                *self.1.lock().unwrap() = Some((text, mode));
                return Ok("Connection 'hs1' (uuid) successfully imported.\n".into());
            }
            self.0.run(program, args)
        }
    }
    let capture = Arc::new(Capture(script.clone(), Mutex::new(None)));
    let gateway = NmVpn::new(
        Nm::new(capture.clone()),
        Systemctl::new(capture.clone()),
        std::env::temp_dir().join(format!("hs-vpn-import-{}", std::process::id())),
    );
    let name = gateway
        .import_wireguard_text("[Interface]\nPrivateKey = k\n")
        .unwrap();
    assert_eq!(name, "hs1");
    let (text, mode) = capture.1.lock().unwrap().clone().unwrap();
    assert!(text.contains("PrivateKey = k"));
    assert_eq!(mode, 0o600);
}

#[test]
fn finalizing_an_import_renames_and_names_the_interface() {
    let script = Arc::new(Script::default());
    script.on(
        "nmcli connection modify id hs1 connection.id Office VPN connection.autoconnect no connection.interface-name Office-VPN ipv4.method manual ipv4.addresses 10.0.0.2/32 ipv4.dns  wireguard.private-key KEY",
        Ok(""),
    );
    vpn(&script)
        .finalize_import(
            "hs1",
            &VpnDraft {
                name: "Office VPN".into(),
                private_key: "KEY".into(),
                addresses: "10.0.0.2/32".into(),
                ..VpnDraft::default()
            },
        )
        .unwrap();
}
