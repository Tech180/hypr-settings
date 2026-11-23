use settings_domain::network::{AccessPoint, ScanOutcome};

use crate::parse::{leading_number, table};

pub const AP_FIELDS: &str = "IN-USE,SSID,SIGNAL,SECURITY,BSSID,FREQ,CHAN,RATE,MODE,DEVICE";

/// Rows of `nmcli -t -f IN-USE,SSID,... device wifi list`. Dedupe and ordering belong to
/// the application.
pub fn parse_access_points(output: &str) -> Vec<AccessPoint> {
    table(output)
        .into_iter()
        .filter(|row| row.len() >= 10)
        .map(|row| {
            let field = |i: usize| row[i].trim();
            let security = match field(3) {
                "--" => "",
                other => other,
            };
            let requires_password = !security.is_empty()
                && security != "--"
                && !security.to_uppercase().contains("OPEN");
            AccessPoint {
                id: format!("{}|{}", row[1], row[4]),
                ssid: field(1).to_owned(),
                bssid: field(4).to_owned(),
                frequency_mhz: leading_number(field(5)),
                channel: leading_number(field(6)),
                strength: leading_number(field(2)),
                security: if security.is_empty() {
                    if requires_password { "Secured" } else { "Open" }.to_owned()
                } else {
                    security.to_owned()
                },
                connected: field(0) == "*",
                requires_password,
                rate_mbps: leading_number(field(7)),
                mode: field(8).to_owned(),
                device: field(9).to_owned(),
            }
        })
        .collect()
}

/// NetworkManager often refuses rapid rescans; the cached list is still good then.
pub fn classify_scan_error(message: &str) -> ScanOutcome {
    let lower = message.to_lowercase();
    if lower.contains("not allowed") || lower.contains("temporarily") {
        ScanOutcome::Rejected(message.to_owned())
    } else {
        ScanOutcome::Error(message.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = include_str!("../tests/fixtures/wifi-list.txt");

    #[test]
    fn rows_become_access_points() {
        let aps = parse_access_points(LIST);
        assert_eq!(aps.len(), 4);

        let home = &aps[0];
        assert_eq!(home.ssid, "Home:5G");
        assert_eq!(home.bssid, "AA:BB:CC:DD:EE:01");
        assert!(home.connected);
        assert_eq!(home.strength, 82);
        assert_eq!(
            (home.frequency_mhz, home.channel, home.rate_mbps),
            (5180, 36, 540)
        );
        assert!(home.requires_password);
        assert_eq!(home.security, "WPA2 WPA3");

        let cafe = &aps[2];
        assert!(!cafe.requires_password);
        assert_eq!(cafe.security, "Open");

        let hidden = &aps[3];
        assert_eq!(hidden.ssid, "");
        assert!(!hidden.requires_password);
    }

    #[test]
    fn throttled_scans_are_not_errors() {
        assert!(matches!(
            classify_scan_error("Error: Scanning not allowed immediately following previous scan."),
            ScanOutcome::Rejected(_)
        ));
        assert!(matches!(
            classify_scan_error("Error: Wi-Fi device not found"),
            ScanOutcome::Error(_)
        ));
    }
}
