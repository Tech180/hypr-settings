use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use settings_domain::bluetooth::BluetoothPreferences;
use settings_domain::controls::{BrightnessSnapshot, ProfileFlags};
use settings_domain::network::{NetworkPreferences, VpnMirrorStore};
use settings_domain::ports::{
    BluetoothPreferencesStore, BrightnessState, Clock, FeatureFlagStore, NetworkPreferencesStore,
    PreferencesStore, TrialJournal, VpnMirrorStorePort,
};
use settings_domain::{DisplayPreferences, PortError, TrialRecord};

use crate::atomic::write_atomic;

fn io_err(path: &Path, err: std::io::Error) -> PortError {
    PortError::Io(format!("{}: {err}", path.display()))
}

fn invalid(path: &Path, err: serde_json::Error) -> PortError {
    PortError::Invalid(format!("{}: {err}", path.display()))
}

/// Reads a JSON file; a missing file is `None`.
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, PortError> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| invalid(path, e)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_err(path, e)),
    }
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), PortError> {
    let mut text = serde_json::to_string_pretty(value).map_err(|e| invalid(path, e))?;
    text.push('\n');
    write_atomic(path, &text).map_err(|e| io_err(path, e))
}

/// `preferences.json`: unknown fields are ignored, a missing file yields defaults.
pub struct JsonPreferencesStore {
    path: PathBuf,
    /// Read instead while the file does not exist yet.
    legacy: Option<PathBuf>,
}

impl JsonPreferencesStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path, legacy: None }
    }

    /// Starts from the AGS preferences until the first save.
    pub fn with_legacy(mut self, legacy: PathBuf) -> Self {
        self.legacy = Some(legacy);
        self
    }
}

/// `bluetooth/preferences.json`: per-device UI class and developer toggles.
pub struct JsonBluetoothPreferencesStore {
    path: PathBuf,
}

impl JsonBluetoothPreferencesStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl BluetoothPreferencesStore for JsonBluetoothPreferencesStore {
    fn load(&self) -> Result<BluetoothPreferences, PortError> {
        Ok(read_json(&self.path)?.unwrap_or_default())
    }

    fn save(&self, prefs: &BluetoothPreferences) -> Result<(), PortError> {
        write_json(&self.path, prefs)
    }
}

/// Reads `path`, else the AGS file it took over from, else the default.
fn load_with_legacy<T: serde::de::DeserializeOwned + Default>(
    path: &Path,
    legacy: Option<&Path>,
) -> Result<T, PortError> {
    if let Some(value) = read_json(path)? {
        return Ok(value);
    }
    Ok(legacy
        .map(read_json::<T>)
        .and_then(Result::ok)
        .flatten()
        .unwrap_or_default())
}

/// `network/preferences.json`: the Wi-Fi scan policy, seeded from the AGS file.
pub struct JsonNetworkPreferencesStore {
    path: PathBuf,
    legacy: Option<PathBuf>,
}

impl JsonNetworkPreferencesStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path, legacy: None }
    }

    pub fn with_legacy(mut self, legacy: PathBuf) -> Self {
        self.legacy = Some(legacy);
        self
    }
}

impl NetworkPreferencesStore for JsonNetworkPreferencesStore {
    fn load(&self) -> Result<NetworkPreferences, PortError> {
        load_with_legacy::<NetworkPreferences>(&self.path, self.legacy.as_deref())
            .map(NetworkPreferences::normalized)
    }

    fn save(&self, prefs: &NetworkPreferences) -> Result<(), PortError> {
        write_json(&self.path, prefs)
    }
}

/// `network/vpn-profiles.json`: WireGuard fields nmcli cannot read back (no secrets).
pub struct JsonVpnMirrorStore {
    path: PathBuf,
    legacy: Option<PathBuf>,
}

impl JsonVpnMirrorStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path, legacy: None }
    }

    pub fn with_legacy(mut self, legacy: PathBuf) -> Self {
        self.legacy = Some(legacy);
        self
    }
}

impl VpnMirrorStorePort for JsonVpnMirrorStore {
    fn load(&self) -> Result<VpnMirrorStore, PortError> {
        load_with_legacy(&self.path, self.legacy.as_deref())
    }

    fn save(&self, store: &VpnMirrorStore) -> Result<(), PortError> {
        write_json(&self.path, store)
    }
}

impl PreferencesStore for JsonPreferencesStore {
    fn load(&self) -> Result<DisplayPreferences, PortError> {
        if let Some(prefs) = read_json(&self.path)? {
            return Ok(prefs);
        }
        let legacy = self.legacy.as_deref().map(read_json).transpose();
        Ok(legacy.ok().flatten().flatten().unwrap_or_default())
    }

    fn save(&self, prefs: &DisplayPreferences) -> Result<(), PortError> {
        write_json(&self.path, prefs)
    }
}

/// What `feature-flags.json` holds on disk.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct FlagsFile {
    #[serde(default)]
    profiles: ProfileFlags,
}

/// `feature-flags.json`, same shape as the AGS file: `{"profiles": {profile: {connector: flags}}}`.
pub struct JsonFeatureFlagStore {
    path: PathBuf,
    legacy: Option<PathBuf>,
}

impl JsonFeatureFlagStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path, legacy: None }
    }

    pub fn with_legacy(mut self, legacy: PathBuf) -> Self {
        self.legacy = Some(legacy);
        self
    }
}

impl FeatureFlagStore for JsonFeatureFlagStore {
    fn load(&self) -> Result<ProfileFlags, PortError> {
        if let Some(file) = read_json::<FlagsFile>(&self.path)? {
            return Ok(file.profiles);
        }
        let legacy = self
            .legacy
            .as_deref()
            .map(read_json::<FlagsFile>)
            .transpose();
        Ok(legacy
            .ok()
            .flatten()
            .flatten()
            .map(|f| f.profiles)
            .unwrap_or_default())
    }

    fn save(&self, flags: &ProfileFlags) -> Result<(), PortError> {
        let mut cleaned = flags.clone();
        cleaned.retain(|_, outputs| {
            outputs.retain(|_, f| !f.is_empty());
            !outputs.is_empty()
        });
        write_json(&self.path, &FlagsFile { profiles: cleaned })
    }
}

/// `brightness.json`: the targets the bar and the brightness keys write to.
pub struct JsonBrightnessState {
    path: PathBuf,
}

impl JsonBrightnessState {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl BrightnessState for JsonBrightnessState {
    fn write(&self, snapshot: &BrightnessSnapshot) -> Result<(), PortError> {
        write_json(&self.path, snapshot)
    }

    fn read(&self) -> Result<Option<BrightnessSnapshot>, PortError> {
        read_json(&self.path)
    }
}

/// `trial-journal.json`: written before a trial touches anything, removed when it ends.
pub struct FileTrialJournal {
    path: PathBuf,
}

impl FileTrialJournal {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl TrialJournal for FileTrialJournal {
    fn write(&self, record: &TrialRecord) -> Result<(), PortError> {
        write_json(&self.path, record)
    }

    fn read(&self) -> Result<Option<TrialRecord>, PortError> {
        read_json(&self.path)
    }

    fn clear(&self) -> Result<(), PortError> {
        match fs::remove_file(&self.path) {
            Err(e) if e.kind() != ErrorKind::NotFound => Err(io_err(&self.path, e)),
            _ => Ok(()),
        }
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn unix_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

#[cfg(test)]
mod tests {
    use settings_domain::{
        ConnectorName, Criteria, Position, Profile, ProfileName, ProfileOutput, ProfileSet, Scale,
        Transform,
    };

    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hypr-settings-system-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn record() -> TrialRecord {
        let mut profile = Profile::new(ProfileName::parse("home").unwrap());
        profile.outputs.push(ProfileOutput {
            criteria: Criteria::Named("eDP-1".into()),
            enable: Some(true),
            mode: "1920x1080@143.97".parse().ok(),
            position: Some(Position::new(0, 0)),
            scale: Scale::new(1.25).ok(),
            transform: Some(Transform::R90),
            rest_glob: false,
            extra: vec!["adaptive_sync".into(), "on".into()],
        });
        TrialRecord {
            connector: ConnectorName::try_from("eDP-1").unwrap(),
            before: ProfileSet::new(vec![profile]),
            deadline_unix: 1_700_000_030,
        }
    }

    #[test]
    fn journal_round_trips_and_clears() {
        let dir = temp_dir("journal");
        let journal = FileTrialJournal::new(dir.join("nested/trial-journal.json"));
        assert_eq!(journal.read().unwrap(), None);

        journal.write(&record()).unwrap();
        assert_eq!(journal.read().unwrap(), Some(record()));

        journal.clear().unwrap();
        journal.clear().unwrap();
        assert_eq!(journal.read().unwrap(), None);
    }

    #[test]
    fn corrupt_journal_is_reported_not_swallowed() {
        let dir = temp_dir("corrupt");
        let path = dir.join("trial-journal.json");
        fs::write(&path, "{ nope").unwrap();
        assert!(matches!(
            FileTrialJournal::new(path).read(),
            Err(PortError::Invalid(_))
        ));
    }

    #[test]
    fn preferences_default_when_missing_and_ignore_unknown_fields() {
        let dir = temp_dir("prefs");
        let path = dir.join("preferences.json");
        let store = JsonPreferencesStore::new(path.clone());
        assert_eq!(store.load().unwrap(), DisplayPreferences::default());

        fs::write(
            &path,
            r#"{"barHostConnector":"DP-3","customHz":{"DP-3":[144]},"nightKelvin":4000}"#,
        )
        .unwrap();
        let prefs = store.load().unwrap();
        assert_eq!(prefs.bar_host_connector.as_deref(), Some("DP-3"));
        assert_eq!(prefs.custom_hz["DP-3"], vec![144]);
    }

    #[test]
    fn preferences_save_round_trips() {
        let dir = temp_dir("prefs-save");
        let store = JsonPreferencesStore::new(dir.join("preferences.json"));
        let mut prefs = DisplayPreferences::default();
        prefs.custom_hz.insert("DP-3".into(), vec![100]);
        store.save(&prefs).unwrap();
        assert_eq!(store.load().unwrap(), prefs);
    }

    #[test]
    fn preferences_start_from_the_ags_file_until_saved() {
        let dir = temp_dir("prefs-legacy");
        let legacy = dir.join("ags.json");
        fs::write(
            &legacy,
            r#"{"barHostConnector":"DP-2","syncBrightness":false}"#,
        )
        .unwrap();
        let store = JsonPreferencesStore::new(dir.join("new.json")).with_legacy(legacy.clone());
        let mut prefs = store.load().unwrap();
        assert_eq!(prefs.bar_host_connector.as_deref(), Some("DP-2"));
        assert!(!prefs.sync_brightness);

        prefs.sync_brightness = true;
        store.save(&prefs).unwrap();
        fs::remove_file(legacy).unwrap();
        assert!(store.load().unwrap().sync_brightness);
    }

    #[test]
    fn feature_flags_keep_the_ags_shape() {
        let dir = temp_dir("flags");
        let path = dir.join("feature-flags.json");
        fs::write(
            &path,
            r#"{"profiles":{"home":{"DP-3":{"hdr":true,"bitdepth10":false}}}}"#,
        )
        .unwrap();
        let store = JsonFeatureFlagStore::new(path.clone());
        let mut flags = store.load().unwrap();
        assert_eq!(flags["home"]["DP-3"].hdr, Some(true));
        assert_eq!(flags["home"]["DP-3"].ten_bit, Some(false));
        assert_eq!(flags["home"]["DP-3"].vrr, None);

        flags
            .entry("empty".into())
            .or_default()
            .insert("X".into(), Default::default());
        store.save(&flags).unwrap();
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains("\"bitdepth10\": false"), "{text}");
        assert!(!text.contains("empty"), "empty entries are dropped: {text}");
        assert!(!text.contains("vrr"));
    }

    #[test]
    fn brightness_state_matches_what_the_bar_reads() {
        use settings_domain::controls::BrightnessTarget;
        let dir = temp_dir("brightness");
        let path = dir.join("brightness.json");
        let state = JsonBrightnessState::new(path.clone());
        assert_eq!(state.read().unwrap(), None);
        let snapshot = BrightnessSnapshot {
            percent: 60,
            sync: true,
            targets: vec![BrightnessTarget::Internal, BrightnessTarget::Ddc { bus: 5 }],
            host: Some(BrightnessTarget::Internal),
        };
        state.write(&snapshot).unwrap();
        assert_eq!(state.read().unwrap(), Some(snapshot));
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains(r#""kind": "internal""#), "{text}");
        assert!(
            text.contains(r#""kind": "ddc""#) && text.contains(r#""bus": 5"#),
            "{text}"
        );
    }

    #[test]
    fn clock_reports_sane_values() {
        let clock = SystemClock;
        assert!(clock.unix_secs() > 1_600_000_000);
        let a = clock.now();
        clock.sleep(Duration::from_millis(5));
        assert!(clock.now() > a);
    }
}
