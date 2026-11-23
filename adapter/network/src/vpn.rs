use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use settings_domain::PortError;
use settings_domain::network::{Connectivity, VpnConfigKind, VpnDraft, VpnManager, VpnProfile};
use settings_domain::ports::VpnGateway;

use crate::connections::{ConnectionRow, parse_connection_rows, split_list};
use crate::devices::parse_status;
use crate::nm::{Nm, port};
use crate::parse::Show;
use crate::systemd::Systemctl;

const VPN_DEVICE_TYPES: [&str; 4] = ["wireguard", "tun", "tap", "ppp"];

fn is_vpn_type(kind: &str) -> bool {
    kind == "vpn" || kind == "wireguard" || kind.starts_with("vpn")
}

fn type_label(kind: &str, service: &str) -> String {
    let service = service.to_lowercase();
    let label = if kind == "wireguard" || service.contains("wireguard") {
        "WireGuard"
    } else if service.contains("openvpn") {
        "OpenVPN"
    } else if service.contains("openconnect") {
        "OpenConnect"
    } else if service.contains("vpnc") || service.contains("cisco") {
        "Cisco VPN"
    } else if service.contains("l2tp") {
        "L2TP"
    } else if service.contains("pptp") {
        "PPTP"
    } else if service.contains("sstp") {
        "SSTP"
    } else if kind.starts_with("vpn") {
        "VPN"
    } else {
        return if kind.is_empty() { "VPN" } else { kind }.to_owned();
    };
    label.to_owned()
}

fn state_label(active: bool, state_raw: &str, connectivity: Connectivity) -> String {
    if !active {
        return "Disconnected".into();
    }
    match connectivity {
        Connectivity::Limited => return "Connected · Limited".into(),
        Connectivity::Full => return "Connected · Full".into(),
        _ => {}
    }
    if state_raw.contains("connected") {
        "Connected".into()
    } else if state_raw.is_empty() {
        "Active".into()
    } else {
        let trimmed = state_raw.trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ');
        if trimmed.is_empty() {
            "Active"
        } else {
            trimmed
        }
        .to_owned()
    }
}

fn number<T: std::str::FromStr + PartialOrd + Default>(raw: &str) -> Option<T> {
    raw.trim().parse::<T>().ok().filter(|n| *n > T::default())
}

fn peers(raw: &str) -> (usize, String) {
    let parts: Vec<&str> = raw
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .collect();
    (parts.len(), parts.first().copied().unwrap_or("").to_owned())
}

fn address(show: &Show, configured: &str, runtime: &str) -> String {
    let value = show.first(configured);
    if value.is_empty() {
        show.prefixed(runtime)
            .into_iter()
            .next()
            .unwrap_or_default()
    } else {
        value.to_owned()
    }
}

fn connectivity_of(device: &Show) -> Connectivity {
    Connectivity::parse(device.any(&["GENERAL.IP4-CONNECTIVITY", "GENERAL.IP6-CONNECTIVITY"]))
}

fn sort(profiles: &mut [VpnProfile]) {
    profiles.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Tunnels from NetworkManager and `wg-quick` units, and what can be done to them.
pub struct NmVpn {
    nm: Nm,
    systemd: Systemctl,
    runtime_dir: PathBuf,
}

impl NmVpn {
    pub fn new(nm: Nm, systemd: Systemctl, runtime_dir: PathBuf) -> Self {
        Self {
            nm,
            systemd,
            runtime_dir,
        }
    }

    fn device_info(&self, iface: &str) -> Show {
        if iface.is_empty() {
            return Show::default();
        }
        self.nm.device_show(iface).unwrap_or_default()
    }

    fn nm_profile(&self, row: &ConnectionRow) -> VpnProfile {
        let active = !row.device.is_empty();
        let conn = self.nm.connection_show(&row.name).unwrap_or_default();
        let iface = if row.device.is_empty() {
            conn.first("connection.interface-name").to_owned()
        } else {
            row.device.clone()
        };
        let device = if active && !iface.is_empty() {
            self.device_info(&iface)
        } else {
            Show::default()
        };

        let service = [
            conn.first("vpn.service-type"),
            conn.first("connection.type"),
            row.kind.as_str(),
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or("");
        let (peer_count, peer_public_key) = peers(conn.first("wireguard.peers"));
        let connectivity = connectivity_of(&device);
        let dns_configured = conn.first("ipv4.dns");
        let gateway = [
            conn.first("ipv4.gateway"),
            device.first("IP4.GATEWAY"),
            conn.first("ipv6.gateway"),
            device.first("IP6.GATEWAY"),
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or("");

        // A runtime keyfile with a matching wg-quick unit belongs to systemd:
        // disconnecting it through NetworkManager would delete the tunnel.
        let runtime_file =
            row.filename.starts_with("/run/") || row.filename.starts_with("/var/run/");
        let unit = if runtime_file && !iface.is_empty() {
            self.systemd.resolve_wg_unit(&iface)
        } else {
            None
        };
        let manager = if unit.is_some() {
            VpnManager::Systemd
        } else {
            VpnManager::NetworkManager
        };
        let name = conn.first("connection.id");
        let uuid = conn.first("connection.uuid");

        VpnProfile {
            id: row.name.clone(),
            name: if name.is_empty() { &row.name } else { name }.to_owned(),
            uuid: if uuid.is_empty() { &row.uuid } else { uuid }.to_owned(),
            type_label: type_label(&row.kind, service),
            active,
            device: if active { iface.clone() } else { String::new() },
            interface_name: iface,
            autoconnect: !conn
                .first("connection.autoconnect")
                .eq_ignore_ascii_case("no"),
            ipv4_address: if address(&conn, "ipv4.addresses", "IP4.ADDRESS").is_empty() {
                address(&device, "ipv4.addresses", "IP4.ADDRESS")
            } else {
                address(&conn, "ipv4.addresses", "IP4.ADDRESS")
            },
            ipv4_method: conn.first("ipv4.method").to_owned(),
            ipv6_address: if address(&conn, "ipv6.addresses", "IP6.ADDRESS").is_empty() {
                address(&device, "ipv6.addresses", "IP6.ADDRESS")
            } else {
                address(&conn, "ipv6.addresses", "IP6.ADDRESS")
            },
            gateway: gateway.to_owned(),
            dns: if dns_configured.is_empty() {
                device.prefixed("IP4.DNS")
            } else {
                split_list(dns_configured)
            },
            mtu: number(device.any(&["GENERAL.MTU", "wireguard.mtu", "802-3-ethernet.mtu"])),
            listen_port: number(conn.first("wireguard.listen-port")),
            peer_count,
            peer_public_key,
            state_label: state_label(active, device.first("GENERAL.STATE"), connectivity),
            manager,
            editable: manager == VpnManager::NetworkManager,
            systemd_unit: unit.unwrap_or_default(),
        }
    }

    /// A tunnel NetworkManager does not own: a `wg-quick` interface or a stopped unit.
    fn system_tunnel(
        &self,
        iface: &str,
        nm_type: &str,
        row_state: &str,
        device_present: bool,
        unit: Option<String>,
    ) -> VpnProfile {
        let device = if device_present {
            self.device_info(iface)
        } else {
            Show::default()
        };
        let connectivity = connectivity_of(&device);
        let ipv4 = address(&device, "ipv4.addresses", "IP4.ADDRESS");
        let ipv6 = address(&device, "ipv6.addresses", "IP6.ADDRESS");
        let gateway = if device.first("IP4.GATEWAY").is_empty() {
            device.first("IP6.GATEWAY")
        } else {
            device.first("IP4.GATEWAY")
        };
        // After a stray `nmcli connection down` the interface can linger without
        // addresses; keep showing it so it can be reconnected through systemd.
        let nm_active = device_present
            && (!["disconnected", "unmanaged", "unavailable"]
                .iter()
                .any(|s| row_state.to_lowercase().contains(s))
                || !ipv4.is_empty()
                || !ipv6.is_empty());
        let unit_active = unit.as_deref().is_some_and(|u| self.systemd.is_active(u));
        let active = unit_active || nm_active;
        let kind = if nm_type.is_empty() {
            "wireguard"
        } else {
            nm_type
        };
        let state_raw = device.first("GENERAL.STATE");

        VpnProfile {
            id: format!("system:{iface}"),
            name: iface.to_owned(),
            uuid: String::new(),
            type_label: type_label(kind, kind),
            active,
            device: if active {
                iface.to_owned()
            } else {
                String::new()
            },
            interface_name: iface.to_owned(),
            autoconnect: false,
            ipv4_address: ipv4,
            ipv4_method: String::new(),
            ipv6_address: ipv6,
            gateway: gateway.to_owned(),
            dns: device.prefixed("IP4.DNS"),
            mtu: number(device.first("GENERAL.MTU")),
            listen_port: None,
            peer_count: 0,
            peer_public_key: String::new(),
            state_label: match (&unit, active) {
                (Some(_), false) => "Stopped".to_owned(),
                _ => state_label(active, state_raw, connectivity),
            },
            manager: if unit.is_some() {
                VpnManager::Systemd
            } else {
                VpnManager::NetworkManager
            },
            systemd_unit: unit.unwrap_or_default(),
            editable: false,
        }
    }

    fn write_temp_conf(&self, conf: &str) -> Result<PathBuf, PortError> {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        fs::create_dir_all(&self.runtime_dir).map_err(|e| PortError::Io(e.to_string()))?;
        // The file stem becomes the interface name: at most 15 characters.
        let stem = format!(
            "hs{:x}{:x}",
            std::process::id() % 0xfffff,
            COUNTER.fetch_add(1, Ordering::Relaxed) % 0xfff
        );
        let path = self.runtime_dir.join(format!("{stem}.conf"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| PortError::Io(format!("{}: {e}", path.display())))?;
        file.write_all(conf.as_bytes())
            .map_err(|e| PortError::Io(e.to_string()))?;
        Ok(path)
    }

    fn import(&self, kind: &str, path: &str) -> Result<String, PortError> {
        let out = self
            .nm
            .run(&["connection", "import", "type", kind, "file", path])
            .map_err(|e| {
                let lower = e.0.to_lowercase();
                if kind == "openvpn"
                    && (lower.contains("unknown vpn plugin")
                        || lower.contains("openvpn")
                        || lower.contains("no vpn plugin"))
                {
                    PortError::Unavailable(
                        "OpenVPN plugin missing. Rebuild with networkmanager-openvpn.".into(),
                    )
                } else {
                    port(e)
                }
            })?;
        Ok(imported_name(&out))
    }
}

/// nmcli prints `Connection 'name' (uuid) successfully imported.`
pub fn imported_name(output: &str) -> String {
    output
        .split_once("Connection '")
        .and_then(|(_, rest)| rest.split_once('\''))
        .map(|(name, _)| name.to_owned())
        .unwrap_or_default()
}

/// The `nmcli connection modify` settings for the non-peer fields of a draft.
pub fn basics_settings(draft: &VpnDraft) -> Vec<String> {
    let mut args = Vec::new();
    let mut set = |key: &str, value: &str| args.extend([key.to_owned(), value.to_owned()]);
    if !draft.name.trim().is_empty() {
        set("connection.id", draft.name.trim());
    }
    set(
        "connection.autoconnect",
        if draft.autoconnect { "yes" } else { "no" },
    );
    if !draft.interface_name.trim().is_empty() {
        set("connection.interface-name", draft.interface_name.trim());
    }
    if !draft.addresses.trim().is_empty() {
        set("ipv4.method", "manual");
        set("ipv4.addresses", draft.addresses.trim());
    }
    set("ipv4.dns", draft.dns.trim());
    if !draft.mtu.trim().is_empty() {
        set("wireguard.mtu", draft.mtu.trim());
    }
    if !draft.listen_port.trim().is_empty() {
        set("wireguard.listen-port", draft.listen_port.trim());
    }
    if !draft.private_key.trim().is_empty() {
        set("wireguard.private-key", draft.private_key.trim());
    }
    args
}

/// A Linux interface name (at most 15 characters) derived from a tunnel name.
pub fn interface_name_for(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "_=+.-".contains(c) {
                c
            } else {
                '-'
            }
        })
        .take(15)
        .collect();
    cleaned.trim_matches('-').to_owned()
}

impl VpnGateway for NmVpn {
    fn profiles(&self) -> Result<Vec<VpnProfile>, PortError> {
        let rows = self
            .nm
            .run(&[
                "-t",
                "-f",
                "NAME,UUID,TYPE,DEVICE,FILENAME",
                "connection",
                "show",
            ])
            .map(|o| parse_connection_rows(&o))
            .unwrap_or_default();
        let devices = self
            .nm
            .run(&[
                "-t",
                "-f",
                "DEVICE,TYPE,STATE,CONNECTION",
                "device",
                "status",
            ])
            .map(|o| parse_status(&o))
            .unwrap_or_default();
        let wg_units = self.systemd.list_wg_units();

        let mut profiles: Vec<VpnProfile> = rows
            .iter()
            .filter(|r| is_vpn_type(&r.kind))
            .map(|r| self.nm_profile(r))
            .collect();

        let claimed_ifaces: Vec<String> = profiles
            .iter()
            .flat_map(|p| [p.device.clone(), p.interface_name.clone()])
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect();
        let claimed_names: Vec<String> = profiles.iter().map(|p| p.name.to_lowercase()).collect();

        let mut system = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for row in devices
            .iter()
            .filter(|d| VPN_DEVICE_TYPES.contains(&d.kind.as_str()))
        {
            let iface = row.iface.to_lowercase();
            let conn = row.connection.to_lowercase();
            if claimed_ifaces.contains(&iface)
                || (!conn.is_empty() && claimed_names.contains(&conn))
            {
                continue;
            }
            let unit = self.systemd.resolve_wg_unit(&row.iface);
            system.push(self.system_tunnel(&row.iface, &row.kind, &row.state, true, unit));
            seen.push(iface);
        }
        // Stopped units have no interface to find them by.
        for unit in &wg_units {
            let iface = unit.iface.to_lowercase();
            if claimed_ifaces.contains(&iface) || seen.contains(&iface) {
                continue;
            }
            let row = devices.iter().find(|d| d.iface.to_lowercase() == iface);
            let mut profile = self.system_tunnel(
                &unit.iface,
                row.map_or("wireguard", |r| r.kind.as_str()),
                row.map_or(
                    if unit.active {
                        "connected"
                    } else {
                        "disconnected"
                    },
                    |r| r.state.as_str(),
                ),
                row.is_some(),
                Some(unit.unit.clone()),
            );
            profile.manager = VpnManager::Systemd;
            profile.active |= unit.active;
            if !unit.active {
                profile.state_label = "Stopped".into();
            } else if profile.state_label == "Disconnected" {
                profile.state_label = "Connected".into();
            }
            system.push(profile);
            seen.push(iface);
        }

        profiles.extend(system);
        sort(&mut profiles);
        Ok(profiles)
    }

    fn up(&self, id: &str) -> Result<(), PortError> {
        self.nm
            .run(&["connection", "up", "id", id])
            .map(drop)
            .map_err(port)
    }

    fn down(&self, id: &str) -> Result<(), PortError> {
        self.nm
            .run(&["connection", "down", "id", id])
            .map(drop)
            .map_err(port)
    }

    fn delete(&self, id: &str) -> Result<(), PortError> {
        self.nm
            .run(&["connection", "delete", "id", id])
            .map(drop)
            .map_err(port)
    }

    fn import_file(&self, kind: VpnConfigKind, path: &str) -> Result<String, PortError> {
        self.import(
            match kind {
                VpnConfigKind::WireGuard => "wireguard",
                VpnConfigKind::OpenVpn => "openvpn",
            },
            path,
        )
    }

    fn import_wireguard_text(&self, conf: &str) -> Result<String, PortError> {
        let path = self.write_temp_conf(conf)?;
        let result = self.import("wireguard", &path.to_string_lossy());
        if let Err(err) = fs::remove_file(&path) {
            tracing::warn!(path = %path.display(), %err, "could not remove the temporary VPN conf");
        }
        result
    }

    fn modify_basics(&self, id: &str, draft: &VpnDraft) -> Result<(), PortError> {
        self.nm.modify(id, &basics_settings(draft))
    }

    fn private_key(&self, id: &str) -> Result<String, PortError> {
        Ok(self
            .nm
            .connection_secrets(id)
            .map(|s| s.first("wireguard.private-key").to_owned())
            .unwrap_or_default())
    }

    fn uuid_of(&self, name: &str) -> Result<String, PortError> {
        Ok(self
            .nm
            .show(&[
                "-t",
                "-f",
                "connection.uuid",
                "connection",
                "show",
                "id",
                name,
            ])
            .map(|s| s.first("connection.uuid").to_owned())
            .unwrap_or_default())
    }

    fn finalize_import(&self, imported_name: &str, draft: &VpnDraft) -> Result<(), PortError> {
        let target = if draft.name.trim().is_empty() {
            imported_name.to_owned()
        } else {
            draft.name.trim().to_owned()
        };
        let mut draft = VpnDraft {
            name: target.clone(),
            ..draft.clone()
        };
        // The import named the interface after the temporary file.
        if draft.interface_name.trim().is_empty() {
            draft.interface_name = interface_name_for(&target);
        }
        let mut settings = Vec::new();
        if target != imported_name {
            settings.extend(["connection.id".to_owned(), target]);
        }
        settings.extend(basics_settings(&VpnDraft {
            name: String::new(),
            ..draft
        }));
        self.nm.modify(imported_name, &settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_imported_name_is_read_from_nmcli_output() {
        assert_eq!(
            imported_name("Connection 'wg-home' (3f1c) successfully imported.\n"),
            "wg-home"
        );
        assert_eq!(imported_name("done"), "");
    }

    #[test]
    fn basics_cover_the_form_and_clear_dns() {
        let draft = VpnDraft {
            name: " Office ".into(),
            interface_name: "wg-office".into(),
            private_key: "KEY".into(),
            addresses: "10.0.0.2/32".into(),
            mtu: "1380".into(),
            listen_port: "51820".into(),
            autoconnect: true,
            ..VpnDraft::default()
        };
        assert_eq!(
            basics_settings(&draft),
            [
                "connection.id",
                "Office",
                "connection.autoconnect",
                "yes",
                "connection.interface-name",
                "wg-office",
                "ipv4.method",
                "manual",
                "ipv4.addresses",
                "10.0.0.2/32",
                "ipv4.dns",
                "",
                "wireguard.mtu",
                "1380",
                "wireguard.listen-port",
                "51820",
                "wireguard.private-key",
                "KEY",
            ]
        );
    }

    #[test]
    fn interface_names_fit_the_kernel_limit() {
        assert_eq!(interface_name_for("Office VPN"), "Office-VPN");
        assert_eq!(
            interface_name_for("a very long tunnel name"),
            "a-very-long-tun"
        );
        assert_eq!(interface_name_for("  "), "");
    }

    #[test]
    fn labels_and_states() {
        assert_eq!(type_label("wireguard", "wireguard"), "WireGuard");
        assert_eq!(
            type_label("vpn", "org.freedesktop.NetworkManager.openvpn"),
            "OpenVPN"
        );
        assert_eq!(type_label("vpn", ""), "VPN");
        assert_eq!(
            state_label(false, "100 (connected)", Connectivity::Full),
            "Disconnected"
        );
        assert_eq!(
            state_label(true, "100 (connected)", Connectivity::Full),
            "Connected · Full"
        );
        assert_eq!(
            state_label(true, "40 (connecting)", Connectivity::Unknown),
            "(connecting)"
        );
    }
}
