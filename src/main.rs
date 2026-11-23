//! Composition root: the only place that knows every adapter.

use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::Arc;

use std::path::PathBuf;

use adapter_bluetooth::{BluerEvents, BluerExtras, BluerGateway};
use adapter_hardware::{BrightnessctlBacklight, DdcutilDdc, HyprsunsetNightLight};
use adapter_hyprland::{HyprEvents, HyprctlLiveOutputs, HyprctlTopology};
use adapter_kanshi::{KanshiConfFile, Kanshictl};
use adapter_network::{
    FsVpnFiles, JournalReader, Nm, NmEvents, NmGateway, NmVpn, QrCodeEncoder, SysfsStats,
    SystemExec, Systemctl,
};
use adapter_system::{
    FileTrialJournal, JsonBluetoothPreferencesStore, JsonBrightnessState, JsonFeatureFlagStore,
    JsonNetworkPreferencesStore, JsonPreferencesStore, JsonVpnMirrorStore, SysfsHardwareProbe,
    SystemClock, XdgPaths,
};
use clap::Parser;
use presentation_cli::{Cli, Command, run_bluetooth, run_display, run_network};
use presentation_gtk::Launch;
use settings_application::{
    BluetoothPorts, BluetoothService, DisplayService, NetworkPorts, NetworkService, Ports,
    SettingsApi, SettingsService,
};
use settings_domain::PortError;
use settings_domain::TrialPolicy;
use settings_domain::ports::{Subscription, TopologyEvent, TopologyEvents};
use tracing_subscriber::EnvFilter;

/// Stands in for the event stream when no Hyprland session is reachable (e.g. a TTY).
struct NoEvents(PortError);

impl TopologyEvents for NoEvents {
    fn subscribe(
        &self,
        _sink: Box<dyn Fn(TopologyEvent) + Send>,
    ) -> Result<Subscription, PortError> {
        Err(PortError::Unavailable(self.0.to_string()))
    }
}

fn compose_network(paths: &XdgPaths) -> NetworkService {
    let exec = Arc::new(SystemExec);
    let nm = Nm::new(exec.clone());
    let systemd = Systemctl::new(exec.clone());
    let gateway = Arc::new(NmGateway::new(nm.clone()));
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("hypr-settings");
    NetworkService::new(NetworkPorts {
        gateway: gateway.clone(),
        connections: gateway,
        vpn: Arc::new(NmVpn::new(nm, systemd.clone(), runtime)),
        units: Arc::new(systemd),
        events: Arc::new(NmEvents::default()),
        stats: Arc::new(SysfsStats::new()),
        journal: Arc::new(JournalReader::new(exec)),
        qr: Arc::new(QrCodeEncoder),
        prefs: Arc::new(
            JsonNetworkPreferencesStore::new(paths.network_preferences_file())
                .with_legacy(paths.legacy_ags_file("network-preferences.json")),
        ),
        mirror: Arc::new(
            JsonVpnMirrorStore::new(paths.vpn_mirror_file())
                .with_legacy(paths.legacy_ags_file("vpn-profiles.json")),
        ),
        files: Arc::new(FsVpnFiles::for_home(
            home,
            std::env::var_os("XDG_DOWNLOAD_DIR").map(PathBuf::from),
        )),
        clock: Arc::new(SystemClock),
    })
}

fn compose() -> Arc<dyn SettingsApi> {
    let paths = XdgPaths::from_env();
    let events: Arc<dyn TopologyEvents> = match HyprEvents::from_env() {
        Ok(events) => Arc::new(events),
        Err(err) => Arc::new(NoEvents(err)),
    };
    let ports = Ports {
        topology: Arc::new(HyprctlTopology::new()),
        events,
        profiles: Arc::new(KanshiConfFile::new(
            paths.kanshi_conf.clone(),
            paths.kanshi_seeds.clone(),
        )),
        controller: Arc::new(Kanshictl::new()),
        live: Arc::new(HyprctlLiveOutputs::new()),
        prefs: Arc::new(
            JsonPreferencesStore::new(paths.preferences_file())
                .with_legacy(paths.legacy_file("preferences.json")),
        ),
        journal: Arc::new(FileTrialJournal::new(paths.trial_journal_file())),
        clock: Arc::new(SystemClock),
        flags: Arc::new(
            JsonFeatureFlagStore::new(paths.feature_flags_file())
                .with_legacy(paths.legacy_file("feature-flags.json")),
        ),
        probe: Arc::new(SysfsHardwareProbe::new()),
        backlight: Arc::new(BrightnessctlBacklight::new()),
        ddc: Arc::new(DdcutilDdc::new()),
        night: Arc::new(HyprsunsetNightLight::new()),
        brightness: Arc::new(JsonBrightnessState::new(paths.brightness_file())),
    };
    let display = DisplayService::new(ports, TrialPolicy::default());
    let gateway = Arc::new(BluerGateway::new());
    let prefs_path = paths.bluetooth_preferences_file();
    let bluetooth = BluetoothService::new(BluetoothPorts {
        gateway: gateway.clone(),
        events: Arc::new(BluerEvents::new(gateway)),
        extras: Arc::new(BluerExtras::new(
            prefs_path.clone(),
            PathBuf::from("/etc/bluetooth/main.conf"),
        )),
        prefs: Arc::new(JsonBluetoothPreferencesStore::new(prefs_path)),
    });
    SettingsService::new(display, bluetooth, compose_network(&paths))
}

fn cli_result(result: Result<(), presentation_cli::CliError>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("hypr-settings: {err}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let filter =
        EnvFilter::try_from_env("HYPR_SETTINGS_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();

    let cli = Cli::parse();
    let launch = match cli.command {
        Some(Command::Display { command }) => {
            let mut out = io::stdout().lock();
            let result = run_display(compose().as_ref(), command, &mut out);
            let _ = out.flush();
            return cli_result(result);
        }
        Some(Command::Bluetooth { command }) => {
            let mut out = io::stdout().lock();
            let result = run_bluetooth(compose().as_ref(), command, &mut out);
            let _ = out.flush();
            return cli_result(result);
        }
        Some(Command::Network { command }) => {
            let mut out = io::stdout().lock();
            let result = run_network(compose().as_ref(), command, &mut out);
            let _ = out.flush();
            return cli_result(result);
        }
        None => Launch::Open { section: None },
        Some(Command::Open { section }) => Launch::Open { section },
        Some(Command::Toggle) => Launch::Toggle,
        Some(Command::Close) => Launch::Close,
        Some(Command::Refresh) => Launch::Refresh,
    };
    presentation_gtk::run(compose(), launch)
}
