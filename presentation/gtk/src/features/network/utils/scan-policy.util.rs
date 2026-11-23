use settings_domain::network::{NetworkPreferences, SCAN_INTERVALS_MS, ScanPolicy};

const DEFAULT_INTERVAL_MS: u32 = 60_000;

pub fn scan_policy_labels() -> Vec<String> {
    let mut labels = vec!["Manual only".to_owned(), "On open".to_owned()];
    labels.extend(SCAN_INTERVALS_MS.iter().map(|ms| {
        let secs = ms / 1000;
        if secs < 120 {
            format!("Every {secs}s")
        } else {
            format!("Every {}m", secs / 60)
        }
    }));
    labels
}

/// Which dropdown row the saved preference selects.
pub fn scan_policy_index(prefs: &NetworkPreferences) -> usize {
    match prefs.policy {
        ScanPolicy::Manual => 0,
        ScanPolicy::OnOpen => 1,
        ScanPolicy::Periodic => {
            2 + SCAN_INTERVALS_MS
                .iter()
                .position(|ms| *ms == prefs.interval_ms)
                .unwrap_or_else(|| {
                    SCAN_INTERVALS_MS
                        .iter()
                        .position(|ms| *ms == DEFAULT_INTERVAL_MS)
                        .unwrap_or(0)
                })
        }
    }
}

/// The policy and interval a dropdown row stands for; the interval stays what it was for
/// the rows that do not use one.
pub fn scan_policy_from_index(index: usize, current_interval_ms: u32) -> (ScanPolicy, u32) {
    match index {
        0 => (ScanPolicy::Manual, current_interval_ms),
        1 => (ScanPolicy::OnOpen, current_interval_ms),
        n => (
            ScanPolicy::Periodic,
            SCAN_INTERVALS_MS
                .get(n - 2)
                .copied()
                .unwrap_or(DEFAULT_INTERVAL_MS),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs(policy: ScanPolicy, interval_ms: u32) -> NetworkPreferences {
        NetworkPreferences {
            version: 1,
            policy,
            interval_ms,
        }
    }

    #[test]
    fn rows_cover_manual_open_and_each_interval() {
        let labels = scan_policy_labels();
        assert_eq!(labels.len(), 2 + SCAN_INTERVALS_MS.len());
        assert_eq!(labels[2], "Every 30s");
        assert_eq!(labels[4], "Every 2m");
    }

    #[test]
    fn index_and_policy_round_trip() {
        for index in 0..scan_policy_labels().len() {
            let (policy, interval) = scan_policy_from_index(index, 60_000);
            assert_eq!(scan_policy_index(&prefs(policy, interval)), index);
        }
    }

    #[test]
    fn unknown_interval_falls_back_to_a_minute() {
        assert_eq!(scan_policy_index(&prefs(ScanPolicy::Periodic, 7)), 3);
    }
}
