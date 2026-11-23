use std::io::Write;

use settings_application::SettingsApi;
use settings_application::network_dto::{PrimaryLink, ScanResult};
use settings_domain::network::ScanReason;

use crate::args::{NetworkCommand, VpnCommand, WifiCommand};
use crate::display::CliError;

pub fn run(
    api: &dyn SettingsApi,
    command: NetworkCommand,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    match command {
        NetworkCommand::Status => status(api, out),
        NetworkCommand::Wifi { command } => wifi(api, command, out),
        NetworkCommand::Vpn { command } => vpn(api, command, out),
    }
}

fn status(api: &dyn SettingsApi, out: &mut dyn Write) -> Result<(), CliError> {
    let overview = api.network_overview()?;
    let link = match overview.primary {
        PrimaryLink::Wifi => "wifi",
        PrimaryLink::Wired => "ethernet",
        PrimaryLink::Offline => "offline",
    };
    writeln!(out, "link: {link}")?;
    writeln!(out, "name: {}", overview.title)?;
    writeln!(out, "connectivity: {:?}", overview.connectivity)?;
    writeln!(
        out,
        "wifi: {}",
        if !overview.wifi_present {
            "no adapter"
        } else if overview.wifi_enabled {
            "on"
        } else {
            "off"
        }
    )?;
    writeln!(
        out,
        "vpn: {} of {} active",
        overview.vpn_active, overview.vpn_total
    )?;
    Ok(())
}

fn wifi(api: &dyn SettingsApi, command: WifiCommand, out: &mut dyn Write) -> Result<(), CliError> {
    match command {
        WifiCommand::List => {
            let view = api.wifi_view()?;
            for ap in &view.access_points {
                writeln!(
                    out,
                    "{}{}  {}%  {}",
                    if ap.connected { "* " } else { "  " },
                    ap.ssid,
                    ap.strength,
                    if ap.requires_password {
                        ap.security.as_str()
                    } else {
                        "open"
                    }
                )?;
            }
        }
        WifiCommand::On => {
            api.set_wifi_enabled(true)?;
            writeln!(out, "wifi on")?;
        }
        WifiCommand::Off => {
            api.set_wifi_enabled(false)?;
            writeln!(out, "wifi off")?;
        }
        WifiCommand::Scan => {
            let result = api.request_wifi_scan(ScanReason::Manual, true, true)?;
            let text = match result {
                ScanResult::Scanned => "scanned".to_owned(),
                ScanResult::Listed => "listed known networks".to_owned(),
                ScanResult::Throttled => "throttled by NetworkManager".to_owned(),
                ScanResult::Skipped(why) => format!("skipped ({})", why.label()),
            };
            writeln!(out, "scan: {text}")?;
        }
        WifiCommand::Connect { ssid, password } => {
            api.connect_wifi(ssid.clone(), password)?;
            writeln!(out, "connected to {ssid}")?;
        }
        WifiCommand::Disconnect => {
            api.disconnect_wifi()?;
            writeln!(out, "disconnected")?;
        }
    }
    Ok(())
}

fn vpn(api: &dyn SettingsApi, command: VpnCommand, out: &mut dyn Write) -> Result<(), CliError> {
    match command {
        VpnCommand::List => {
            for profile in api.vpn_view()?.profiles {
                writeln!(
                    out,
                    "{}{}  {}  {}",
                    if profile.active { "* " } else { "  " },
                    profile.id,
                    profile.type_label,
                    profile.state_label
                )?;
            }
        }
        VpnCommand::Up { id } => {
            api.activate_vpn(id.clone())?;
            writeln!(out, "{id} up")?;
        }
        VpnCommand::Down { id } => {
            api.deactivate_vpn(id.clone())?;
            writeln!(out, "{id} down")?;
        }
    }
    Ok(())
}
