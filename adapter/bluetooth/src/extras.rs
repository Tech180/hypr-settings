use std::path::PathBuf;
use std::process::Command;

use settings_domain::PortError;
use settings_domain::bluetooth::BluetoothPreferences;
use settings_domain::ports::BluetoothExtras;

/// PipeWire / config file extras and JSON preference merge.
pub struct BluerExtras {
    prefs: PathBuf,
    bluez_main_conf: PathBuf,
}

impl BluerExtras {
    pub fn new(prefs: PathBuf, bluez_main_conf: PathBuf) -> Self {
        Self {
            prefs,
            bluez_main_conf,
        }
    }

    fn load_prefs(&self) -> Result<BluetoothPreferences, PortError> {
        Ok(read_json(&self.prefs)?.unwrap_or_default())
    }

    fn save_prefs(&self, prefs: &BluetoothPreferences) -> Result<(), PortError> {
        write_json(&self.prefs, prefs)
    }
}

impl BluetoothExtras for BluerExtras {
    fn pan_enabled(&self) -> Result<bool, PortError> {
        Ok(self.load_prefs()?.developer.pan_enabled)
    }

    fn set_pan_enabled(&self, on: bool) -> Result<(), PortError> {
        let mut prefs = self.load_prefs()?;
        prefs.developer.pan_enabled = on;
        self.save_prefs(&prefs)
    }

    fn avrcp_versions(&self) -> Result<Vec<String>, PortError> {
        Ok(vec![
            "AVRCP 1.3".into(),
            "AVRCP 1.4".into(),
            "AVRCP 1.5 (Default)".into(),
            "AVRCP 1.6".into(),
        ])
    }

    fn avrcp_version(&self) -> Result<Option<String>, PortError> {
        Ok(self.load_prefs()?.developer.avrcp_version.clone())
    }

    fn set_avrcp_version(&self, version: &str) -> Result<(), PortError> {
        let mut prefs = self.load_prefs()?;
        prefs.developer.avrcp_version = Some(version.to_owned());
        self.save_prefs(&prefs)?;
        patch_main_conf(&self.bluez_main_conf, "AvrcpVersion", version)
    }

    fn sample_rates(&self) -> Result<Vec<u32>, PortError> {
        Ok(vec![44_100, 48_000, 88_200, 96_000])
    }

    fn sample_rate_hz(&self) -> Result<Option<u32>, PortError> {
        Ok(self.load_prefs()?.developer.sample_rate_hz)
    }

    fn set_sample_rate_hz(&self, hz: u32) -> Result<(), PortError> {
        let mut prefs = self.load_prefs()?;
        prefs.developer.sample_rate_hz = Some(hz);
        self.save_prefs(&prefs)
    }

    fn a2dp_offload_disabled(&self) -> Result<bool, PortError> {
        Ok(self.load_prefs()?.developer.a2dp_offload_disabled)
    }

    fn set_a2dp_offload_disabled(&self, disabled: bool) -> Result<(), PortError> {
        let mut prefs = self.load_prefs()?;
        prefs.developer.a2dp_offload_disabled = disabled;
        self.save_prefs(&prefs)
    }

    fn hci_snoop_enabled(&self) -> Result<bool, PortError> {
        Ok(self.load_prefs()?.developer.hci_snoop)
    }

    fn set_hci_snoop_enabled(&self, on: bool) -> Result<(), PortError> {
        let mut prefs = self.load_prefs()?;
        prefs.developer.hci_snoop = on;
        self.save_prefs(&prefs)
    }

    fn clear_gatt_cache(&self) -> Result<(), PortError> {
        let script = "menu gatt\nclear\nback\nquit\n";
        let output = Command::new("bluetoothctl")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(script.as_bytes())?;
                }
                child.wait()
            });
        match output {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(PortError::Unavailable(format!(
                "bluetoothctl exited with {status}"
            ))),
            Err(e) => Err(PortError::Unavailable(e.to_string())),
        }
    }

    fn auracast_available(&self) -> bool {
        std::fs::read_to_string("/etc/bluetooth/main.conf")
            .map(|text| text.contains("Experimental = true") || text.contains("Experimental=true"))
            .unwrap_or(false)
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<Option<T>, PortError> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| PortError::Invalid(format!("{}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(PortError::Io(format!("{}: {e}", path.display()))),
    }
}

fn write_json<T: serde::Serialize>(path: &PathBuf, value: &T) -> Result<(), PortError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| PortError::Io(format!("{}: {e}", parent.display())))?;
    }
    let mut text =
        serde_json::to_string_pretty(value).map_err(|e| PortError::Invalid(e.to_string()))?;
    text.push('\n');
    std::fs::write(path, text).map_err(|e| PortError::Io(format!("{}: {e}", path.display())))
}

fn patch_main_conf(path: &PathBuf, key: &str, value: &str) -> Result<(), PortError> {
    if !path.exists() {
        return Err(PortError::Unavailable(format!(
            "bluez config missing at {}",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(path).map_err(|e| PortError::Io(e.to_string()))?;
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let needle = format!("{key} =");
    let replacement = format!("{key} = {value}");
    let mut found = false;
    for line in &mut lines {
        if line.trim_start().starts_with(&needle) {
            *line = replacement.clone();
            found = true;
        }
    }
    if !found {
        lines.push(replacement);
    }
    let out = lines.join("\n") + "\n";
    std::fs::write(path, out).map_err(|e| PortError::Io(e.to_string()))
}
