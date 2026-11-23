use std::collections::HashSet;

/// A Wi-Fi profile NetworkManager already knows (`connection.id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedWifiProfile {
    pub id: String,
    pub ssid: String,
}

/// One visible Wi-Fi network, as the radio reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessPoint {
    pub id: String,
    pub ssid: String,
    pub bssid: String,
    pub frequency_mhz: u32,
    pub channel: u32,
    /// Signal strength, 0 to 100.
    pub strength: u32,
    pub security: String,
    pub connected: bool,
    pub requires_password: bool,
    pub rate_mbps: u32,
    pub mode: String,
    pub device: String,
}

impl AccessPoint {
    /// Signal as 0 to 4 bars, for the icon.
    pub fn signal_bars(&self) -> u8 {
        match self.strength {
            0 => 0,
            1..=24 => 1,
            25..=49 => 2,
            50..=74 => 3,
            _ => 4,
        }
    }
}

/// True when joining needs a secret (anything but an open network).
pub fn security_requires_password(security: &str) -> bool {
    let sec = security.trim();
    !sec.is_empty() && sec != "--" && !sec.to_uppercase().contains("OPEN")
}

pub fn band_label(frequency_mhz: u32) -> &'static str {
    match frequency_mhz {
        0 => "—",
        1..=2999 => "2.4 GHz",
        3000..=5924 => "5 GHz",
        _ => "6 GHz",
    }
}

/// Connected first, then strongest; one row per SSID; hidden networks are dropped.
pub fn normalize_access_points(mut items: Vec<AccessPoint>) -> Vec<AccessPoint> {
    items.sort_by(|a, b| {
        b.connected
            .cmp(&a.connected)
            .then(b.strength.cmp(&a.strength))
            .then_with(|| a.ssid.cmp(&b.ssid))
    });
    let mut seen = HashSet::new();
    items.retain(|ap| !ap.ssid.is_empty() && seen.insert(ap.ssid.clone()));
    items
}

/// Covers every field a card shows, so an unchanged scan can skip the rebuild.
pub fn access_points_signature(items: &[AccessPoint]) -> String {
    items
        .iter()
        .map(|ap| {
            format!(
                "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                ap.id,
                ap.ssid,
                ap.bssid,
                u8::from(ap.connected),
                u8::from(ap.requires_password),
                ap.security,
                ap.strength,
                ap.frequency_mhz,
                ap.channel,
                ap.rate_mbps,
                ap.device,
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ap(ssid: &str, strength: u32, connected: bool) -> AccessPoint {
        AccessPoint {
            id: format!("{ssid}|aa"),
            ssid: ssid.into(),
            bssid: "aa".into(),
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

    #[test]
    fn connected_first_then_strongest_one_row_per_ssid() {
        let list = normalize_access_points(vec![
            ap("weak", 20, false),
            ap("strong", 90, false),
            ap("strong", 30, false),
            ap("home", 10, true),
            ap("", 99, false),
        ]);
        let names: Vec<_> = list.iter().map(|a| a.ssid.as_str()).collect();
        assert_eq!(names, ["home", "strong", "weak"]);
        assert_eq!(list[1].strength, 90);
    }

    #[test]
    fn open_networks_need_no_password() {
        assert!(!security_requires_password(""));
        assert!(!security_requires_password("--"));
        assert!(!security_requires_password("Open"));
        assert!(security_requires_password("WPA2"));
    }

    #[test]
    fn bands_and_bars() {
        assert_eq!(band_label(2412), "2.4 GHz");
        assert_eq!(band_label(5180), "5 GHz");
        assert_eq!(band_label(6115), "6 GHz");
        assert_eq!(band_label(0), "—");
        assert_eq!(ap("a", 0, false).signal_bars(), 0);
        assert_eq!(ap("a", 60, false).signal_bars(), 3);
        assert_eq!(ap("a", 100, false).signal_bars(), 4);
    }

    #[test]
    fn signature_changes_with_visible_fields_only() {
        let a = vec![ap("x", 50, false)];
        let mut b = a.clone();
        assert_eq!(access_points_signature(&a), access_points_signature(&b));
        b[0].strength = 51;
        assert_ne!(access_points_signature(&a), access_points_signature(&b));
    }
}
