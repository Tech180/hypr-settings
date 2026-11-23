use super::*;

#[test]
fn custom_rates_are_sorted_unique_and_bounded() {
    let mut prefs = DisplayPreferences::default();
    assert!(prefs.add_custom_hz("DP-3", 144));
    assert!(prefs.add_custom_hz("DP-3", 100));
    assert!(!prefs.add_custom_hz("DP-3", 100), "duplicate");
    assert!(!prefs.add_custom_hz("DP-3", 0));
    assert!(!prefs.add_custom_hz("DP-3", 1001));
    assert_eq!(prefs.custom_hz_of("DP-3"), [100, 144]);
}

#[test]
fn removing_the_last_rate_forgets_the_connector() {
    let mut prefs = DisplayPreferences::default();
    prefs.add_custom_hz("DP-3", 120);
    assert!(prefs.remove_custom_hz("DP-3", 120));
    assert!(!prefs.remove_custom_hz("DP-3", 120));
    assert!(prefs.custom_hz.is_empty());
}

#[test]
fn old_ags_files_load_with_new_defaults() {
    let prefs: DisplayPreferences =
        serde_json::from_str(r#"{"barHostConnector":"DP-3","customHz":{"DP-3":[75]}}"#).unwrap();
    assert_eq!(prefs.bar_host_connector.as_deref(), Some("DP-3"));
    assert_eq!(prefs.custom_hz_of("DP-3"), [75]);
    assert!(prefs.sync_brightness);
}
