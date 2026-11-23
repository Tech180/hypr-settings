use settings_domain::network::{DeviceActionOutcome, DeviceKind, NetworkDevice};

use crate::parse::{Show, table};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRow {
    pub iface: String,
    pub kind: String,
    pub state: String,
    pub connection: String,
}

pub fn parse_status(output: &str) -> Vec<DeviceRow> {
    table(output)
        .into_iter()
        .filter(|row| row.len() >= 2)
        .map(|row| {
            let at = |i: usize| row.get(i).cloned().unwrap_or_default();
            DeviceRow {
                iface: at(0),
                kind: at(1),
                state: at(2),
                connection: at(3),
            }
        })
        .collect()
}

pub fn type_matches(kind: DeviceKind, nm_type: &str) -> bool {
    match kind {
        DeviceKind::Wifi => nm_type == "wifi",
        DeviceKind::Ethernet => nm_type == "ethernet",
    }
}

fn parse_number<T: std::str::FromStr>(text: &str) -> Option<T> {
    text.trim().parse().ok()
}

fn parse_speed(raw: &str) -> Option<u32> {
    if raw.is_empty() || raw == "unknown" {
        return None;
    }
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// A device from `nmcli -t -f all device show <iface>`.
pub fn device_from_show(iface: &str, kind: DeviceKind, show: &Show) -> NetworkDevice {
    let state = show.first("GENERAL.STATE").to_lowercase();
    let connected = state.contains("(connected)") || state.starts_with("100");
    let first_address = |prefix: &str| {
        show.prefixed(prefix)
            .first()
            .and_then(|a| a.split('/').next())
            .unwrap_or("")
            .to_owned()
    };
    let model = [show.first("GENERAL.PRODUCT"), show.first("GENERAL.VENDOR")]
        .into_iter()
        .find(|v| !v.is_empty())
        .map_or_else(
            || match kind {
                DeviceKind::Wifi => "Wi-Fi Adapter".to_owned(),
                DeviceKind::Ethernet => "Ethernet Adapter".to_owned(),
            },
            str::to_owned,
        );

    NetworkDevice {
        iface: iface.to_owned(),
        kind,
        model,
        enabled: !state.contains("unavailable") && !state.contains("unmanaged"),
        connected,
        speed_mbps: parse_speed(show.first("CAPABILITIES.SPEED")),
        mtu: parse_number(show.first("GENERAL.MTU")),
        rx_bytes: parse_number(show.any(&["GENERAL.RX-BYTES", "IP4.RX-BYTES"])),
        tx_bytes: parse_number(show.any(&["GENERAL.TX-BYTES", "IP4.TX-BYTES"])),
        mac: show.first("GENERAL.HWADDR").to_owned(),
        ip: first_address("IP4.ADDRESS"),
        gateway: show.first("IP4.GATEWAY").to_owned(),
        dns: show.prefixed("IP4.DNS"),
        ipv6: first_address("IP6.ADDRESS"),
        connection_name: show.first("GENERAL.CONNECTION").to_owned(),
        internet_access: connected,
    }
}

pub fn classify_connect_error(message: &str) -> DeviceActionOutcome {
    let lower = message.to_lowercase();
    if lower.contains("no carrier") || lower.contains("not available on device") {
        DeviceActionOutcome::NoCarrier(message.to_owned())
    } else {
        DeviceActionOutcome::Error(message.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &str = include_str!("../tests/fixtures/device-status.txt");
    const ETH: &str = include_str!("../tests/fixtures/device-show-eth.txt");

    #[test]
    fn status_rows_are_filtered_by_type() {
        let rows = parse_status(STATUS);
        let wired: Vec<_> = rows
            .iter()
            .filter(|r| type_matches(DeviceKind::Ethernet, &r.kind))
            .collect();
        assert_eq!(wired.len(), 1);
        assert_eq!(wired[0].iface, "enp3s0");
        assert_eq!(wired[0].connection, "Wired connection 1");
        assert!(rows.iter().any(|r| type_matches(DeviceKind::Wifi, &r.kind)));
    }

    #[test]
    fn show_output_becomes_a_device() {
        let device = device_from_show("enp3s0", DeviceKind::Ethernet, &Show::parse(ETH));
        assert!(device.connected && device.enabled && device.internet_access);
        assert_eq!(
            device.model,
            "RTL8111/8168/8411 PCI Express Gigabit Ethernet Controller"
        );
        assert_eq!(device.speed_mbps, Some(1000));
        assert_eq!(device.mtu, Some(1500));
        assert_eq!(device.mac, "AA:BB:CC:00:11:22");
        assert_eq!(device.ip, "192.168.1.20");
        assert_eq!(device.gateway, "192.168.1.1");
        assert_eq!(device.dns, ["192.168.1.1", "1.1.1.1"]);
        assert_eq!(device.ipv6, "fe80::1");
        assert_eq!(device.connection_name, "Wired connection 1");
    }

    #[test]
    fn unplugged_cable_is_not_enabled() {
        let show = Show::parse("GENERAL.STATE:20 (unavailable)\nCAPABILITIES.SPEED:unknown\n");
        let device = device_from_show("enp3s0", DeviceKind::Ethernet, &show);
        assert!(!device.enabled && !device.connected);
        assert_eq!(device.speed_mbps, None);
        assert_eq!(device.model, "Ethernet Adapter");
    }

    #[test]
    fn connect_errors_are_classified() {
        assert!(matches!(
            classify_connect_error("Error: Connection activation failed: No carrier"),
            DeviceActionOutcome::NoCarrier(_)
        ));
        assert!(matches!(
            classify_connect_error("Error: something else"),
            DeviceActionOutcome::Error(_)
        ));
    }
}
