use settings_application::network_dto::WifiSavedEntry;
use settings_domain::network::AccessPoint;

use super::{filter_access_points, filter_saved_connections};

fn ap(ssid: &str, connected: bool) -> AccessPoint {
    AccessPoint {
        id: ssid.into(),
        ssid: ssid.into(),
        bssid: String::new(),
        frequency_mhz: 0,
        channel: 0,
        strength: 50,
        security: String::new(),
        connected,
        requires_password: false,
        rate_mbps: 0,
        mode: String::new(),
        device: String::new(),
    }
}

#[test]
fn search_matches_names_ignoring_case() {
    let items = vec![
        ap("Home", true),
        ap("Cafe Wifi", false),
        ap("Office", false),
    ];
    assert_eq!(filter_access_points(&items, "").len(), 3);
    let found = filter_access_points(&items, "  CAFE ");
    let names: Vec<_> = found.iter().map(|a| a.ssid.as_str()).collect();
    assert_eq!(names, ["Cafe Wifi"]);
    assert_eq!(filter_access_points(&items, "zzz").len(), 0);
}

#[test]
fn saved_search_matches_ssid_or_profile_name() {
    let saved = vec![
        WifiSavedEntry {
            id: "Home-5G".into(),
            ssid: "Home".into(),
            active: true,
        },
        WifiSavedEntry {
            id: "Office".into(),
            ssid: "CorpNet".into(),
            active: false,
        },
    ];
    assert_eq!(filter_saved_connections(&saved, "").len(), 2);
    assert_eq!(filter_saved_connections(&saved, "corp").len(), 1);
    assert_eq!(filter_saved_connections(&saved, "5g").len(), 1);
}
