use std::path::PathBuf;

/// Where hypr-settings keeps its state and finds kanshi's files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgPaths {
    /// `$XDG_STATE_HOME/hypr-settings/display/` (override `HYPR_SETTINGS_STATE_DIR`).
    pub state_dir: PathBuf,
    /// The conf kanshi is launched against (override `HYPR_SETTINGS_KANSHI_CONF`).
    pub kanshi_conf: PathBuf,
    /// Candidates for seeding a missing kanshi conf, most specific first.
    pub kanshi_seeds: Vec<PathBuf>,
    /// The AGS display state this app took over from; read once when a file is missing.
    pub legacy_state_dir: PathBuf,
}

impl XdgPaths {
    pub fn from_env() -> Self {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let non_empty = |key: &str| get(key).filter(|v| !v.is_empty());
        let home = non_empty("HOME").map(PathBuf::from).unwrap_or_default();
        let state_home = non_empty("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/state"));
        let config_home = non_empty("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));

        let state_dir = non_empty("HYPR_SETTINGS_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| state_home.join("hypr-settings/display"));
        let kanshi_conf = non_empty("HYPR_SETTINGS_KANSHI_CONF")
            .map(PathBuf::from)
            .unwrap_or_else(|| state_home.join("hypr-settings/display/kanshi.conf"));

        Self {
            state_dir,
            kanshi_conf,
            legacy_state_dir: state_home.join("ags/display"),
            kanshi_seeds: vec![
                state_home.join("ags/display/kanshi.conf"),
                config_home.join("kanshi/config"),
            ],
        }
    }

    pub fn preferences_file(&self) -> PathBuf {
        self.state_dir.join("preferences.json")
    }

    pub fn feature_flags_file(&self) -> PathBuf {
        self.state_dir.join("feature-flags.json")
    }

    /// Read by the bar and the brightness keys.
    pub fn brightness_file(&self) -> PathBuf {
        self.state_dir.join("brightness.json")
    }

    pub fn legacy_file(&self, name: &str) -> PathBuf {
        self.legacy_state_dir.join(name)
    }

    pub fn trial_journal_file(&self) -> PathBuf {
        self.state_dir.join("trial-journal.json")
    }

    /// Bluetooth UI overrides and developer toggles.
    pub fn bluetooth_preferences_file(&self) -> PathBuf {
        self.bluetooth_state_dir().join("preferences.json")
    }

    pub fn bluetooth_state_dir(&self) -> PathBuf {
        self.state_dir
            .parent()
            .map(|p| p.join("bluetooth"))
            .unwrap_or_else(|| PathBuf::from(".local/state/hypr-settings/bluetooth"))
    }

    pub fn network_state_dir(&self) -> PathBuf {
        self.state_dir
            .parent()
            .map(|p| p.join("network"))
            .unwrap_or_else(|| PathBuf::from(".local/state/hypr-settings/network"))
    }

    /// Scan policy and interval.
    pub fn network_preferences_file(&self) -> PathBuf {
        self.network_state_dir().join("preferences.json")
    }

    /// Non-secret WireGuard fields nmcli cannot read back.
    pub fn vpn_mirror_file(&self) -> PathBuf {
        self.network_state_dir().join("vpn-profiles.json")
    }

    /// A file of the AGS shell's state root (`~/.local/state/ags/<name>`).
    pub fn legacy_ags_file(&self, name: &str) -> PathBuf {
        self.legacy_state_dir
            .parent()
            .map(|p| p.join(name))
            .unwrap_or_else(|| PathBuf::from(name))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn paths(vars: &[(&str, &str)]) -> XdgPaths {
        let env: HashMap<_, _> = vars.iter().copied().collect();
        XdgPaths::from_lookup(|k| env.get(k).map(|v| (*v).to_owned()))
    }

    #[test]
    fn defaults_follow_the_xdg_spec() {
        let p = paths(&[("HOME", "/home/u")]);
        assert_eq!(
            p.state_dir,
            PathBuf::from("/home/u/.local/state/hypr-settings/display")
        );
        assert_eq!(
            p.kanshi_conf,
            PathBuf::from("/home/u/.local/state/hypr-settings/display/kanshi.conf")
        );
        assert_eq!(
            p.kanshi_seeds,
            [
                PathBuf::from("/home/u/.local/state/ags/display/kanshi.conf"),
                PathBuf::from("/home/u/.config/kanshi/config"),
            ]
        );
    }

    #[test]
    fn xdg_variables_and_overrides_win() {
        let p = paths(&[
            ("HOME", "/home/u"),
            ("XDG_STATE_HOME", "/s"),
            ("XDG_CONFIG_HOME", "/c"),
            ("HYPR_SETTINGS_KANSHI_CONF", "/tmp/k.conf"),
        ]);
        assert_eq!(p.state_dir, PathBuf::from("/s/hypr-settings/display"));
        assert_eq!(p.kanshi_conf, PathBuf::from("/tmp/k.conf"));
        assert_eq!(p.kanshi_seeds[1], PathBuf::from("/c/kanshi/config"));
        assert_eq!(
            p.trial_journal_file(),
            PathBuf::from("/s/hypr-settings/display/trial-journal.json")
        );

        let p = paths(&[("HOME", "/h"), ("HYPR_SETTINGS_STATE_DIR", "/x")]);
        assert_eq!(p.preferences_file(), PathBuf::from("/x/preferences.json"));
    }
}
