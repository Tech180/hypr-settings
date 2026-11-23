use settings_domain::network::{
    AccessPoint, Connectivity, DeviceKind, LinkState, NetworkDevice, band_label,
};

use crate::shared::icon::Icon;

const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

/// `1.2 MB`, `850 B`; powers of 1024.
pub fn format_bytes(bytes: u64) -> String {
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// `1.2 MB/s`; a dash while there is no rate yet.
pub fn format_rate(bytes_per_sec: Option<u64>) -> String {
    bytes_per_sec.map_or_else(|| "—".to_owned(), |b| format!("{}/s", format_bytes(b)))
}

pub fn wifi_icon(bars: u8) -> Icon {
    match bars {
        0 | 1 => Icon::WifiLow,
        2 => Icon::SignalLow,
        3 => Icon::WifiHigh,
        _ => Icon::Wifi,
    }
}

/// `5 GHz`, plus the Wi-Fi generation when the rate hints at one.
pub fn band_text(frequency_mhz: u32) -> String {
    band_label(frequency_mhz).to_owned()
}

/// Short security label for a header badge.
pub fn security_badge(ap: &AccessPoint) -> String {
    if !ap.requires_password {
        return "Open".to_owned();
    }
    let sec = ap.security.trim();
    if sec.is_empty() || sec == "--" {
        "Secured".to_owned()
    } else {
        sec.to_owned()
    }
}

/// The line under a network's name (signal strength is shown on the Wi-Fi icon).
pub fn ap_subtitle(ap: &AccessPoint) -> String {
    if ap.connected {
        if ap.requires_password {
            "Connected, secured".to_owned()
        } else {
            "Connected, open".to_owned()
        }
    } else if ap.requires_password {
        "Secured".to_owned()
    } else {
        "Open network".to_owned()
    }
}

pub fn link_text(state: LinkState, kind: DeviceKind) -> &'static str {
    match (state, kind) {
        (LinkState::Connected, _) => "Connected",
        (LinkState::Disconnected, _) => "Not connected",
        (LinkState::Unavailable, DeviceKind::Ethernet) => "Cable unplugged",
        (LinkState::Unavailable, DeviceKind::Wifi) => "Unavailable",
    }
}

pub fn connectivity_text(connectivity: Connectivity) -> &'static str {
    match connectivity {
        Connectivity::Full => "Connected to Internet",
        Connectivity::Limited => "Limited connectivity",
        Connectivity::None => "No Internet access",
        Connectivity::Unknown => "Connected",
    }
}

pub fn device_label(device: &NetworkDevice) -> String {
    device.label()
}

/// `HH:MM:SS` for a unix time, in the local zone.
pub fn clock_text(unix_secs: u64) -> String {
    i64::try_from(unix_secs)
        .ok()
        .and_then(|secs| gtk4::glib::DateTime::from_unix_local(secs).ok())
        .and_then(|time| time.format("%H:%M:%S").ok())
        .map_or_else(|| "--:--:--".to_owned(), |text| text.to_string())
}

/// `192.168.1.5 / 24`, or just the address when there is no prefix.
#[cfg_attr(not(test), allow(dead_code))]
pub fn address_with_prefix(address: &str, prefix: &str) -> String {
    match (address.is_empty(), prefix.is_empty()) {
        (true, _) => "—".to_owned(),
        (false, true) => address.to_owned(),
        (false, false) => format!("{address} / {prefix}"),
    }
}

pub fn dns_text(dns: &[String]) -> String {
    if dns.is_empty() {
        "—".to_owned()
    } else {
        dns.join(", ")
    }
}

pub fn or_dash(text: &str) -> String {
    if text.trim().is_empty() {
        "—".to_owned()
    } else {
        text.to_owned()
    }
}

/// The part of the journal line after the host name, so lines are short enough to read.
pub fn journal_text(line: &str) -> (String, String) {
    // `2026-01-01T12:00:00+0000 host NetworkManager[123]: <info>  message`
    let mut parts = line.splitn(2, ' ');
    let stamp = parts.next().unwrap_or("");
    if !stamp.contains('T') {
        return (String::new(), line.trim().to_owned());
    }
    let rest = parts.next().unwrap_or("");
    let time = stamp
        .split('T')
        .nth(1)
        .map_or(stamp, |t| t.split(['+', '-', 'Z']).next().unwrap_or(t));
    let message = rest.split_once(": ").map_or(rest, |(_, m)| m);
    (time.to_owned(), message.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_scale_by_1024() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(format_rate(None), "—");
        assert_eq!(format_rate(Some(2048)), "2.0 KB/s");
    }

    #[test]
    fn addresses_and_dns_read_naturally() {
        assert_eq!(address_with_prefix("10.0.0.2", "24"), "10.0.0.2 / 24");
        assert_eq!(address_with_prefix("10.0.0.2", ""), "10.0.0.2");
        assert_eq!(address_with_prefix("", "24"), "—");
        assert_eq!(dns_text(&[]), "—");
        assert_eq!(
            dns_text(&["1.1.1.1".into(), "9.9.9.9".into()]),
            "1.1.1.1, 9.9.9.9"
        );
        assert_eq!(or_dash("  "), "—");
    }

    #[test]
    fn journal_lines_split_time_and_message() {
        let (time, message) = journal_text(
            "2026-05-01T14:32:01+0200 techtop NetworkManager[812]: <info>  [1.2] device (wlan0): state change",
        );
        assert_eq!(time, "14:32:01");
        assert_eq!(message, "<info>  [1.2] device (wlan0): state change");
        assert_eq!(journal_text("-- no entries --").1, "-- no entries --");
    }

    #[test]
    fn subtitles_say_what_the_network_is() {
        let ap = |connected, requires_password| AccessPoint {
            id: String::new(),
            ssid: "x".into(),
            bssid: String::new(),
            frequency_mhz: 5180,
            channel: 36,
            strength: 60,
            security: "WPA2".into(),
            connected,
            requires_password,
            rate_mbps: 100,
            mode: String::new(),
            device: String::new(),
        };
        assert_eq!(ap_subtitle(&ap(true, true)), "Connected, secured");
        assert_eq!(ap_subtitle(&ap(false, false)), "Open network");
    }
}
