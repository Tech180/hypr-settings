use clap::Parser;
use settings_application::dto::BluetoothView;
use settings_application::dto::{
    ApplyOutcome, DisplayView, HzChip, OutputView, ProfileView, RecoverOutcome, ResolutionChip,
    ScaleChip, TickOutcome, TransformChip, TrialView,
};
use settings_application::dto::{BrightnessView, NightLightView, OutputControls};
use settings_application::network_dto::{
    DiagnosticsView, EthernetView, NetworkOverview, NetworkRates, ScanResult, VpnView,
    WifiProfileView, WifiView,
};
use settings_application::{
    AppError, BluetoothApi, ControlsApi, DisplayApi, NetworkApi, SettingsApi,
};
use settings_domain::bluetooth::DeviceClass;
use settings_domain::bluetooth::DeviceId;
use settings_domain::bluetooth::ProfileKind;
use settings_domain::controls::Flag;
use settings_domain::network::{
    AccessPoint, ConnectionProfile, IpConfig, IpVersion, NetworkPreferences, QrMatrix, ScanPolicy,
    ScanReason, VpnCandidate, VpnDraft,
};
use settings_domain::ports::{Subscription, TopologyEvent};
use settings_domain::{
    ConnectorName, DisplayKind, LayoutPatch, ModeRequest, ProfileName, Scale, Transform,
};

use crate::{Cli, Command, DisplayCommand, build_patch, run_display};

struct Stub {
    recover: RecoverOutcome,
}

fn view() -> DisplayView {
    DisplayView {
        outputs: vec![OutputView {
            connector: "eDP-1".into(),
            description: "BOE 0x0A89".into(),
            make: "BOE".into(),
            model: "0x0A89".into(),
            kind: DisplayKind::Laptop,
            enabled: true,
            focused: true,
            ultrawide: false,
            width: 1920,
            height: 1080,
            refresh_hz: 60,
            x: 0,
            y: 0,
            logical_width: 1280,
            logical_height: 720,
            flipped: false,
            suggested_x: 0,
            suggested_y: 0,
            resolutions: vec![ResolutionChip {
                width: 1920,
                height: 1080,
                active: true,
            }],
            hz_options: vec![HzChip {
                hz: 60,
                active: true,
                custom: false,
            }],
            scale_options: vec![ScaleChip {
                hundredths: 150,
                label: "1.5×".into(),
                active: true,
            }],
            transform_options: vec![TransformChip {
                hypr: 0,
                label: "0°",
                active: true,
            }],
        }],
        profiles: vec![
            ProfileView {
                name: "portable".into(),
                output_count: 1,
                active: true,
                removable: true,
                outputs: Vec::new(),
            },
            ProfileView {
                name: "default".into(),
                output_count: 1,
                active: false,
                removable: false,
                outputs: Vec::new(),
            },
        ],
        active_profile: Some("portable".into()),
        bar_host: Some("eDP-1".into()),
        trial: None,
        warnings: Vec::new(),
    }
}

impl DisplayApi for Stub {
    fn view(&self) -> Result<DisplayView, AppError> {
        Ok(view())
    }
    fn begin_trial(&self, _: LayoutPatch) -> Result<TrialView, AppError> {
        unreachable!()
    }
    fn keep_trial(&self) -> Result<(), AppError> {
        Ok(())
    }
    fn revert_trial(&self) -> Result<(), AppError> {
        Ok(())
    }
    fn tick(&self) -> Result<TickOutcome, AppError> {
        Ok(TickOutcome::Idle)
    }
    fn set_enabled(&self, _: ConnectorName, _: bool) -> Result<ApplyOutcome, AppError> {
        Ok(ApplyOutcome::Verified)
    }
    fn switch_profile(&self, _: ProfileName) -> Result<(), AppError> {
        Ok(())
    }
    fn save_snapshot_as(&self, _: ProfileName) -> Result<(), AppError> {
        Ok(())
    }
    fn save_to_active_profile(&self) -> Result<String, AppError> {
        Ok("portable".into())
    }
    fn set_bar_host(&self, _: ConnectorName) -> Result<(), AppError> {
        Ok(())
    }
    fn add_custom_hz(&self, _: ConnectorName, _: u32) -> Result<(), AppError> {
        Ok(())
    }
    fn remove_custom_hz(&self, _: ConnectorName, _: u32) -> Result<(), AppError> {
        Ok(())
    }
    fn delete_profile(&self, _: ProfileName) -> Result<(), AppError> {
        Ok(())
    }
    fn recover_pending_trial(&self) -> Result<RecoverOutcome, AppError> {
        Ok(self.recover.clone())
    }
    fn subscribe(&self, _: Box<dyn Fn(TopologyEvent) + Send>) -> Result<Subscription, AppError> {
        Ok(Subscription::new(|| {}))
    }
}

impl ControlsApi for Stub {
    fn output_controls(&self, _: ConnectorName) -> Result<OutputControls, AppError> {
        unreachable!()
    }
    fn brightness(&self) -> Result<BrightnessView, AppError> {
        Ok(BrightnessView {
            percent: Some(40),
            sync: true,
            targets: 2,
        })
    }
    fn refresh_brightness(&self) -> Result<BrightnessView, AppError> {
        self.brightness()
    }
    fn set_brightness(&self, _: u8) -> Result<(), AppError> {
        Ok(())
    }
    fn adjust_brightness(&self, delta: i32) -> Result<u8, AppError> {
        Ok(u8::try_from(40 + delta).unwrap_or(0))
    }
    fn set_sync_brightness(&self, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn set_contrast(&self, _: ConnectorName, _: u8) -> Result<(), AppError> {
        Ok(())
    }
    fn set_overdrive(&self, _: ConnectorName, _: u8) -> Result<(), AppError> {
        Ok(())
    }
    fn set_input_source(&self, _: ConnectorName, _: u8) -> Result<(), AppError> {
        Ok(())
    }
    fn set_flag(&self, _: ConnectorName, _: Flag, _: bool) -> Result<ApplyOutcome, AppError> {
        Ok(ApplyOutcome::Verified)
    }
    fn restore_flags(&self) -> Result<usize, AppError> {
        Ok(2)
    }
    fn night_light(&self) -> Result<NightLightView, AppError> {
        Ok(NightLightView {
            on: true,
            kelvin: 4200,
        })
    }
    fn set_night_light(&self, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn set_night_kelvin(&self, _: u32) -> Result<(), AppError> {
        Ok(())
    }
    fn restore_night_light(&self) -> Result<(), AppError> {
        Ok(())
    }
}

impl BluetoothApi for Stub {
    fn bluetooth_view(&self) -> Result<BluetoothView, AppError> {
        Ok(BluetoothView::default())
    }
    fn set_bluetooth_powered(&self, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn start_bluetooth_discovery(&self) -> Result<(), AppError> {
        Ok(())
    }
    fn stop_bluetooth_discovery(&self) -> Result<(), AppError> {
        Ok(())
    }
    fn select_bluetooth_adapter(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
    fn pair_device(&self, _: DeviceId) -> Result<(), AppError> {
        Ok(())
    }
    fn connect_device(&self, _: DeviceId) -> Result<(), AppError> {
        Ok(())
    }
    fn disconnect_device(&self, _: DeviceId) -> Result<(), AppError> {
        Ok(())
    }
    fn forget_device(&self, _: DeviceId) -> Result<(), AppError> {
        Ok(())
    }
    fn set_device_class(&self, _: DeviceId, _: DeviceClass) -> Result<(), AppError> {
        Ok(())
    }
    fn set_device_profile(&self, _: DeviceId, _: ProfileKind, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn set_device_codec(&self, _: DeviceId, _: String) -> Result<(), AppError> {
        Ok(())
    }
    fn set_absolute_volume(&self, _: DeviceId, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn set_pan_enabled(&self, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn set_avrcp_version(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
    fn set_sample_rate_hz(&self, _: u32) -> Result<(), AppError> {
        Ok(())
    }
    fn set_a2dp_offload_disabled(&self, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn set_hci_snoop_enabled(&self, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn clear_gatt_cache(&self) -> Result<(), AppError> {
        Ok(())
    }
    fn subscribe_bluetooth(&self, _: Box<dyn Fn() + Send>) -> Result<Subscription, AppError> {
        Ok(Subscription::new(|| {}))
    }
}

fn refused<T>() -> Result<T, AppError> {
    Err(AppError::Rejected("stub".into()))
}

impl NetworkApi for Stub {
    fn network_overview(&self) -> Result<NetworkOverview, AppError> {
        Ok(NetworkOverview {
            primary: settings_application::network_dto::PrimaryLink::Wifi,
            title: "Home".into(),
            connectivity: settings_domain::network::Connectivity::Full,
            wifi_present: true,
            wifi_enabled: true,
            wifi_ssid: Some("Home".into()),
            ethernet_present: false,
            ethernet_connected: false,
            vpn_total: 2,
            vpn_active: 1,
            has_wifi_profile: true,
        })
    }
    fn network_rates(&self) -> NetworkRates {
        NetworkRates::default()
    }
    fn wifi_view(&self) -> Result<WifiView, AppError> {
        let ap = |ssid: &str, connected: bool, requires_password: bool| AccessPoint {
            id: ssid.into(),
            ssid: ssid.into(),
            bssid: String::new(),
            frequency_mhz: 5180,
            channel: 36,
            strength: 70,
            security: "WPA2".into(),
            connected,
            requires_password,
            rate_mbps: 300,
            mode: String::new(),
            device: "wlan0".into(),
        };
        Ok(WifiView {
            enabled: true,
            devices: Vec::new(),
            selected: "wlan0".into(),
            access_points: vec![ap("Home", true, true), ap("Cafe", false, false)],
            saved: Vec::new(),
            signature: String::new(),
            scanning: false,
        })
    }
    fn ethernet_view(&self) -> Result<EthernetView, AppError> {
        refused()
    }
    fn vpn_view(&self) -> Result<VpnView, AppError> {
        refused()
    }
    fn wifi_profile(&self) -> Result<WifiProfileView, AppError> {
        refused()
    }
    fn connection_profile(&self, _: String) -> Result<Option<ConnectionProfile>, AppError> {
        Ok(None)
    }
    fn network_diagnostics(&self) -> Result<DiagnosticsView, AppError> {
        refused()
    }
    fn set_wifi_enabled(&self, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn select_wifi_adapter(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
    fn request_wifi_scan(&self, _: ScanReason, _: bool, _: bool) -> Result<ScanResult, AppError> {
        Ok(ScanResult::Scanned)
    }
    fn connect_wifi(&self, _: String, _: Option<String>) -> Result<(), AppError> {
        Ok(())
    }
    fn activate_wifi_connection(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
    fn disconnect_wifi(&self) -> Result<(), AppError> {
        Ok(())
    }
    fn set_ethernet_enabled(&self, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn select_ethernet_adapter(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
    fn set_connection_autoconnect(&self, _: String, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn set_connection_metered(&self, _: String, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn set_connection_mac_random(&self, _: String, _: bool) -> Result<(), AppError> {
        Ok(())
    }
    fn apply_ip_settings(&self, _: String, _: IpVersion, _: IpConfig) -> Result<(), AppError> {
        Ok(())
    }
    fn forget_connection(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
    fn wifi_qr(&self, _: String) -> Result<QrMatrix, AppError> {
        refused()
    }
    fn activate_vpn(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
    fn deactivate_vpn(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
    fn load_vpn_draft(&self, _: String) -> Result<VpnDraft, AppError> {
        refused()
    }
    fn save_vpn_draft(&self, _: VpnDraft) -> Result<(), AppError> {
        Ok(())
    }
    fn delete_vpn(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
    fn import_vpn(&self, _: String) -> Result<String, AppError> {
        refused()
    }
    fn vpn_candidates(&self) -> Vec<VpnCandidate> {
        Vec::new()
    }
    fn network_preferences(&self) -> Result<NetworkPreferences, AppError> {
        Ok(NetworkPreferences::default())
    }
    fn set_scan_policy(&self, _: ScanPolicy, _: u32) -> Result<(), AppError> {
        Ok(())
    }
    fn clear_network_log(&self) {}
    fn subscribe_network(&self, _: Box<dyn Fn() + Send>) -> Result<Subscription, AppError> {
        Ok(Subscription::new(|| {}))
    }
}

fn capture(command: DisplayCommand, recover: RecoverOutcome) -> String {
    let mut out = Vec::new();
    run_display(&Stub { recover } as &dyn SettingsApi, command, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("hypr-settings").chain(args.iter().copied())).unwrap()
}

fn set_args(args: &[&str]) -> crate::SetArgs {
    match parse(&[&["display", "set"], args].concat()).command {
        Some(Command::Display {
            command: DisplayCommand::Set(a),
        }) => a,
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn list_prints_a_readable_table() {
    let text = capture(
        DisplayCommand::List { json: false },
        RecoverOutcome::Nothing,
    );
    assert!(text.contains("eDP-1"));
    assert!(text.contains("1920x1080@60"));
    assert!(text.contains("scale 1.5×"));
    assert!(text.contains("(focused)"));
    assert!(text.contains("profile: portable"));
}

#[test]
fn list_json_is_machine_readable() {
    let text = capture(DisplayCommand::List { json: true }, RecoverOutcome::Nothing);
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc["activeProfile"], "portable");
    assert_eq!(doc["outputs"][0]["connector"], "eDP-1");
    assert_eq!(doc["outputs"][0]["scale"], 1.5);
    assert_eq!(doc["outputs"][0]["transform"], 0);
    assert_eq!(doc["outputs"][0]["hzOptions"][0], 60);
}

#[test]
fn profiles_mark_the_active_one() {
    let text = capture(
        DisplayCommand::Profiles { json: false },
        RecoverOutcome::Nothing,
    );
    assert!(text.contains("* portable"));
    assert!(text.contains("  default"));
    let json = capture(
        DisplayCommand::Profiles { json: true },
        RecoverOutcome::Nothing,
    );
    let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(doc[0]["active"], true);
}

#[test]
fn revert_pending_reports_what_happened() {
    assert_eq!(
        capture(DisplayCommand::RevertPending, RecoverOutcome::Nothing),
        "no pending trial\n"
    );
    let text = capture(
        DisplayCommand::RevertPending,
        RecoverOutcome::Restored {
            connector: "DP-3".into(),
        },
    );
    assert!(text.contains("DP-3"));
}

#[test]
fn set_flags_become_a_patch() {
    let patch = build_patch(&set_args(&[
        "--connector",
        "eDP-1",
        "--scale",
        "1.5",
        "--rotate",
        "90",
        "--mode",
        "1920x1080@60",
    ]))
    .unwrap();
    assert_eq!(patch.scale, Some(Scale::new(1.5).unwrap()));
    assert_eq!(patch.transform, Some(Transform::R90));
    assert!(matches!(patch.mode, Some(ModeRequest::Exact(_))));

    let patch = build_patch(&set_args(&["--connector", "DP-3", "--mode", "1920x1080"])).unwrap();
    assert!(matches!(patch.mode, Some(ModeRequest::Resolution(_))));

    let patch = build_patch(&set_args(&["--connector", "DP-3", "--refresh", "144"])).unwrap();
    assert_eq!(patch.mode, Some(ModeRequest::Refresh(144)));

    let patch = build_patch(&set_args(&["--connector", "DP-3", "--disable"])).unwrap();
    assert_eq!(patch.enable, Some(false));
}

#[test]
fn position_and_flip_flags_become_a_patch() {
    let patch = build_patch(&set_args(&[
        "--connector",
        "DP-3",
        "--x",
        "-1080",
        "--y",
        "0",
        "--rotate",
        "90",
        "--flip",
    ]))
    .unwrap();
    assert_eq!(
        patch.position,
        Some(settings_domain::Position::new(-1080, 0))
    );
    assert_eq!(patch.transform, Some(Transform::Flipped90));

    let patch = build_patch(&set_args(&["--connector", "DP-3", "--flip"])).unwrap();
    assert_eq!(patch.transform, Some(Transform::Flipped));

    // An x without a y is a usage error.
    assert!(
        Cli::try_parse_from([
            "hypr-settings",
            "display",
            "set",
            "--connector",
            "a",
            "--x",
            "1"
        ])
        .is_err()
    );
}

#[test]
fn move_takes_a_connector_and_coordinates() {
    match parse(&["display", "move", "DP-3", "1920", "-200"]).command {
        Some(Command::Display {
            command: DisplayCommand::Move(m),
        }) => assert_eq!((m.connector.as_str(), m.x, m.y), ("DP-3", 1920, -200)),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn bad_flags_are_rejected_with_a_reason() {
    for args in [
        &["--connector", "DP 3", "--enable"][..],
        &["--connector", "DP-3", "--rotate", "45"],
        &["--connector", "DP-3", "--scale", "9"],
        &["--connector", "DP-3", "--mode", "nonsense"],
    ] {
        assert!(build_patch(&set_args(args)).is_err(), "{args:?}");
    }
    assert!(
        Cli::try_parse_from([
            "hypr-settings",
            "display",
            "set",
            "--connector",
            "a",
            "--enable",
            "--disable"
        ])
        .is_err()
    );
}

#[test]
fn window_commands_parse() {
    assert!(parse(&[]).command.is_none());
    assert!(matches!(
        parse(&["open", "--section", "display"]).command,
        Some(Command::Open { section: Some(s) }) if s == "display"
    ));
    assert!(matches!(parse(&["toggle"]).command, Some(Command::Toggle)));
}

#[test]
fn enable_only_goes_through_set_enabled() {
    let mut out = Vec::new();
    let args = set_args(&["--connector", "eDP-1", "--disable"]);
    run_display(
        &Stub {
            recover: RecoverOutcome::Nothing,
        } as &dyn SettingsApi,
        DisplayCommand::Set(args),
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "eDP-1: applied\n");
}

#[test]
fn hardware_commands_parse_and_report() {
    let run = |args: &[&str]| match parse(&[&["display"], args].concat()).command {
        Some(Command::Display { command }) => capture(command, RecoverOutcome::Nothing),
        other => panic!("unexpected {other:?}"),
    };
    assert_eq!(run(&["main", "DP-3"]), "DP-3 hosts the bar now\n");
    assert_eq!(run(&["save"]), "layout stored in profile portable\n");
    assert_eq!(
        run(&["hz", "add", "DP-3", "144"]),
        "DP-3: 144 Hz is offered\n"
    );
    assert_eq!(
        run(&["flag", "DP-3", "10bit", "on"]),
        "DP-3: 10-bit color on\n"
    );
    assert_eq!(run(&["brightness", "adjust", "-5"]), "35%\n");
    assert_eq!(
        run(&["brightness", "get"]),
        "40%  (2 display(s), sync on)\n"
    );
    assert_eq!(run(&["night", "status"]), "night light on at 4200 K\n");
    assert_eq!(run(&["night", "set", "9000"]), "6500 K\n");
    assert_eq!(run(&["input", "DP-3", "0x0f"]), "DP-3: input 0x0f\n");
    assert!(run(&["restore"]).contains("restored on 2 output(s)"));
}

#[test]
fn input_sources_accept_hex_and_decimal() {
    assert_eq!(crate::controls::parse_source("0x0f").unwrap(), 15);
    assert_eq!(crate::controls::parse_source("17").unwrap(), 17);
    assert!(crate::controls::parse_source("hdmi").is_err());
    assert!(crate::controls::parse_source("300").is_err());
}

#[test]
fn network_commands_print_what_the_service_reports() {
    use crate::args::{NetworkCommand, WifiCommand};
    let run = |command: NetworkCommand| {
        let mut out = Vec::new();
        crate::run_network(
            &Stub {
                recover: RecoverOutcome::Nothing,
            } as &dyn SettingsApi,
            command,
            &mut out,
        )
        .unwrap();
        String::from_utf8(out).unwrap()
    };
    let status = run(NetworkCommand::Status);
    assert!(status.contains("link: wifi"), "{status}");
    assert!(status.contains("vpn: 1 of 2 active"), "{status}");
    assert_eq!(
        run(NetworkCommand::Wifi {
            command: WifiCommand::List
        }),
        "* Home  70%  WPA2\n  Cafe  70%  open\n"
    );
    assert_eq!(
        run(NetworkCommand::Wifi {
            command: WifiCommand::Connect {
                ssid: "Cafe".into(),
                password: None
            }
        }),
        "connected to Cafe\n"
    );
    assert_eq!(
        run(NetworkCommand::Wifi {
            command: WifiCommand::Scan
        }),
        "scan: scanned\n"
    );
}
