use std::collections::HashSet;

use settings_application::network_dto::{WifiSavedEntry, WifiView};
use settings_domain::network::AccessPoint;

fn matches_query(haystack: &str, needle: &str) -> bool {
    needle.is_empty() || haystack.to_lowercase().contains(needle)
}

/// Scan results that are not connected and not already in saved profiles.
pub fn available_networks(view: &WifiView) -> Vec<AccessPoint> {
    let saved_ssids: HashSet<_> = view.saved.iter().map(|entry| entry.ssid.as_str()).collect();
    view
        .access_points
        .iter()
        .filter(|ap| !ap.connected && !saved_ssids.contains(ap.ssid.as_str()))
        .cloned()
        .collect()
}

/// Networks whose SSID contains the query, ignoring case.
pub fn filter_access_points(items: &[AccessPoint], query: &str) -> Vec<AccessPoint> {
    let needle = query.trim().to_lowercase();
    items
        .iter()
        .filter(|ap| matches_query(&ap.ssid, &needle))
        .cloned()
        .collect()
}

/// Saved profiles whose SSID or connection name matches the query.
pub fn filter_saved_connections<'a>(
    items: &'a [WifiSavedEntry],
    query: &str,
) -> Vec<&'a WifiSavedEntry> {
    let needle = query.trim().to_lowercase();
    items
        .iter()
        .filter(|entry| {
            matches_query(&entry.ssid, &needle) || matches_query(&entry.id, &needle)
        })
        .collect()
}

#[cfg(test)]
#[path = "wifi-filter.util.test.rs"]
mod tests;
