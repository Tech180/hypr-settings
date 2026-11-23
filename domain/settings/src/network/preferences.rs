use super::scan::{SCAN_INTERVALS_MS, ScanPolicy};

/// `network.json`. The same shape the AGS menu stored, so the old file seeds this one.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPreferences {
    #[serde(default = "version")]
    pub version: u32,
    #[serde(default)]
    pub policy: ScanPolicy,
    #[serde(default = "default_interval")]
    pub interval_ms: u32,
}

fn version() -> u32 {
    1
}

fn default_interval() -> u32 {
    60_000
}

impl Default for NetworkPreferences {
    fn default() -> Self {
        Self {
            version: version(),
            policy: ScanPolicy::default(),
            interval_ms: default_interval(),
        }
    }
}

impl NetworkPreferences {
    /// Snaps an interval that is not on offer (a hand-edited file) to the default.
    pub fn normalized(mut self) -> Self {
        if !SCAN_INTERVALS_MS.contains(&self.interval_ms) {
            self.interval_ms = default_interval();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_ags_file() {
        let json = r#"{"version":1,"policy":"periodic","intervalMs":120000}"#;
        let prefs: NetworkPreferences = serde_json::from_str(json).unwrap();
        assert_eq!(prefs.policy, ScanPolicy::Periodic);
        assert_eq!(prefs.interval_ms, 120_000);
    }

    #[test]
    fn missing_fields_default_and_bad_intervals_snap() {
        let prefs: NetworkPreferences = serde_json::from_str("{}").unwrap();
        assert_eq!(prefs, NetworkPreferences::default());
        let odd = NetworkPreferences {
            interval_ms: 7,
            ..Default::default()
        };
        assert_eq!(odd.normalized().interval_ms, 60_000);
    }

    #[test]
    fn manual_policy_round_trips() {
        let prefs = NetworkPreferences {
            policy: ScanPolicy::Manual,
            ..Default::default()
        };
        let json = serde_json::to_string(&prefs).unwrap();
        assert!(json.contains("\"manual\""), "{json}");
        assert_eq!(
            serde_json::from_str::<NetworkPreferences>(&json).unwrap(),
            prefs
        );
    }
}
