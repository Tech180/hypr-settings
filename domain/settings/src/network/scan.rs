use std::time::Duration;

/// When the Wi-Fi list rescans on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScanPolicy {
    /// Only the refresh button scans.
    Manual,
    /// Once when the page opens.
    #[default]
    OnOpen,
    /// On open, then every configured interval while the page is showing.
    Periodic,
}

/// Intervals the UI offers, in milliseconds.
pub const SCAN_INTERVALS_MS: [u32; 4] = [30_000, 60_000, 120_000, 300_000];

/// Automatic rescans closer together than this are answered from the cached list.
pub const SCAN_HARD_FLOOR_MS: u64 = 15_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanReason {
    Manual,
    Open,
    Periodic,
    AdapterChange,
    RadioEnabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanSkip {
    InFlight,
    Cooldown,
    WifiDisabled,
    PageHidden,
}

impl ScanSkip {
    pub fn label(self) -> &'static str {
        match self {
            Self::InFlight => "in-flight",
            Self::Cooldown => "cooldown",
            Self::WifiDisabled => "wifi-disabled",
            Self::PageHidden => "page-hidden",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanState {
    pub in_flight: bool,
    pub wifi_enabled: bool,
    pub page_visible: bool,
    /// Time since the last scan finished; `None` when none has run.
    pub since_last: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanDecision {
    /// Ask the radio to scan.
    Scan,
    /// Refresh the list from what the radio already knows.
    ListCached(ScanSkip),
    Skip(ScanSkip),
}

/// The single rule for whether a rescan request reaches the radio.
pub fn decide_scan(reason: ScanReason, force: bool, state: &ScanState) -> ScanDecision {
    if state.in_flight {
        return ScanDecision::Skip(ScanSkip::InFlight);
    }
    if !state.wifi_enabled && reason != ScanReason::RadioEnabled {
        return ScanDecision::Skip(ScanSkip::WifiDisabled);
    }
    let needs_page = !matches!(
        reason,
        ScanReason::Manual | ScanReason::RadioEnabled | ScanReason::AdapterChange
    );
    if needs_page && !state.page_visible {
        return ScanDecision::Skip(ScanSkip::PageHidden);
    }
    let floor = Duration::from_millis(SCAN_HARD_FLOOR_MS);
    if !force && state.since_last.is_some_and(|d| d < floor) {
        return ScanDecision::ListCached(ScanSkip::Cooldown);
    }
    ScanDecision::Scan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ScanState {
        ScanState {
            in_flight: false,
            wifi_enabled: true,
            page_visible: true,
            since_last: None,
        }
    }

    #[test]
    fn a_first_scan_runs() {
        assert_eq!(
            decide_scan(ScanReason::Open, false, &state()),
            ScanDecision::Scan
        );
    }

    #[test]
    fn in_flight_wins_even_when_forced() {
        let s = ScanState {
            in_flight: true,
            ..state()
        };
        assert_eq!(
            decide_scan(ScanReason::Manual, true, &s),
            ScanDecision::Skip(ScanSkip::InFlight)
        );
    }

    #[test]
    fn a_disabled_radio_only_scans_after_being_enabled() {
        let s = ScanState {
            wifi_enabled: false,
            ..state()
        };
        assert_eq!(
            decide_scan(ScanReason::Manual, true, &s),
            ScanDecision::Skip(ScanSkip::WifiDisabled)
        );
        assert_eq!(
            decide_scan(ScanReason::RadioEnabled, false, &s),
            ScanDecision::Scan
        );
    }

    #[test]
    fn automatic_scans_wait_for_the_page() {
        let s = ScanState {
            page_visible: false,
            ..state()
        };
        assert_eq!(
            decide_scan(ScanReason::Periodic, false, &s),
            ScanDecision::Skip(ScanSkip::PageHidden)
        );
        assert_eq!(
            decide_scan(ScanReason::Manual, false, &s),
            ScanDecision::Scan
        );
        assert_eq!(
            decide_scan(ScanReason::AdapterChange, false, &s),
            ScanDecision::Scan
        );
    }

    #[test]
    fn cooldown_serves_the_cache_unless_forced() {
        let s = ScanState {
            since_last: Some(Duration::from_secs(5)),
            ..state()
        };
        assert_eq!(
            decide_scan(ScanReason::Open, false, &s),
            ScanDecision::ListCached(ScanSkip::Cooldown)
        );
        assert_eq!(
            decide_scan(ScanReason::Manual, true, &s),
            ScanDecision::Scan
        );
        let old = ScanState {
            since_last: Some(Duration::from_secs(20)),
            ..state()
        };
        assert_eq!(
            decide_scan(ScanReason::Open, false, &old),
            ScanDecision::Scan
        );
    }
}
