//! What the kernel exposes about a connector: `/sys/class/drm/card*-<connector>/` and the
//! DisplayPort AUX device for the DPCD.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use settings_domain::ConnectorName;
use settings_domain::controls::{DPCD_OFFSET_REV, DPCD_READ_LEN, DrmProbe, parse_dpcd};
use settings_domain::ports::HardwareProbe;

const DRM_CLASS: &str = "/sys/class/drm";
const EDID_MIN_LEN: usize = 128;

pub struct SysfsHardwareProbe {
    class_dir: PathBuf,
}

impl SysfsHardwareProbe {
    pub fn new() -> Self {
        Self::at(PathBuf::from(DRM_CLASS))
    }

    pub fn at(class_dir: PathBuf) -> Self {
        Self { class_dir }
    }

    /// `card1-DP-3`; the connected card wins when several carry the connector.
    fn connector_dir(&self, connector: &ConnectorName) -> Option<PathBuf> {
        let suffix = format!("-{connector}");
        let mut found: Vec<PathBuf> = fs::read_dir(&self.class_dir)
            .ok()?
            .flatten()
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                name.starts_with("card") && name.ends_with(&suffix)
            })
            .map(|entry| entry.path())
            .collect();
        found.sort();
        let connected = found
            .iter()
            .position(|dir| read_text(&dir.join("status")).is_some_and(|s| s == "connected"));
        connected
            .map(|i| found.swap_remove(i))
            .or_else(|| found.into_iter().next())
    }
}

impl Default for SysfsHardwareProbe {
    fn default() -> Self {
        Self::new()
    }
}

fn read_text(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|t| t.trim().to_owned())
}

fn read_bool(path: &Path) -> Option<bool> {
    let text = read_text(path)?.to_lowercase();
    match text.as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        other => other.parse::<i64>().ok().map(|n| n != 0),
    }
}

fn aux_device(dir: &Path) -> Option<PathBuf> {
    let name = fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| n.starts_with("drm_dp_aux"))?;
    Some(PathBuf::from("/dev").join(name))
}

/// Reads `len` bytes at `offset` from the AUX character device.
fn read_dpcd(device: &Path, offset: u64, len: usize) -> Option<Vec<u8>> {
    use std::io::{Seek, SeekFrom};
    let mut file = File::open(device).ok()?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut buffer = vec![0u8; len];
    file.read_exact(&mut buffer).ok()?;
    Some(buffer)
}

impl HardwareProbe for SysfsHardwareProbe {
    fn drm(&self, connector: &ConnectorName) -> DrmProbe {
        let Some(dir) = self.connector_dir(connector) else {
            return DrmProbe::default();
        };
        DrmProbe {
            edid: fs::read(dir.join("edid"))
                .ok()
                .filter(|e| e.len() >= EDID_MIN_LEN),
            vrr_capable: read_bool(&dir.join("vrr_capable")),
            max_bpc: read_text(&dir.join("max_bpc")).and_then(|t| t.parse().ok()),
            dpcd: aux_device(&dir)
                .and_then(|device| read_dpcd(&device, DPCD_OFFSET_REV, DPCD_READ_LEN))
                .and_then(|bytes| parse_dpcd(&bytes)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hypr-settings-drm-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn name(value: &str) -> ConnectorName {
        ConnectorName::try_from(value).unwrap()
    }

    #[test]
    fn reads_sysfs_attributes_of_the_connected_card() {
        let root = temp_dir("sysfs");
        let off = root.join("card0-DP-3");
        let on = root.join("card1-DP-3");
        fs::create_dir_all(&off).unwrap();
        fs::create_dir_all(&on).unwrap();
        fs::write(off.join("status"), "disconnected\n").unwrap();
        fs::write(on.join("status"), "connected\n").unwrap();
        fs::write(on.join("vrr_capable"), "1\n").unwrap();
        fs::write(on.join("max_bpc"), "12\n").unwrap();
        fs::write(on.join("edid"), vec![0u8; 256]).unwrap();

        let probe = SysfsHardwareProbe::at(root).drm(&name("DP-3"));
        assert_eq!(probe.vrr_capable, Some(true));
        assert_eq!(probe.max_bpc, Some(12));
        assert_eq!(probe.edid.map(|e| e.len()), Some(256));
        assert_eq!(probe.dpcd, None);
    }

    #[test]
    fn unknown_connectors_and_missing_files_give_nothing() {
        let root = temp_dir("missing");
        fs::create_dir_all(root.join("card0-HDMI-A-1")).unwrap();
        let probe = SysfsHardwareProbe::at(root.clone());
        assert_eq!(probe.drm(&name("DP-9")), DrmProbe::default());
        assert_eq!(probe.drm(&name("HDMI-A-1")), DrmProbe::default());
        assert_eq!(
            SysfsHardwareProbe::at(root.join("nope")).drm(&name("DP-3")),
            DrmProbe::default()
        );
    }

    #[test]
    fn dpcd_is_read_at_the_given_offset() {
        let root = temp_dir("dpcd");
        let device = root.join("aux");
        fs::write(&device, [0xaa, 0x14, 0x1e, 0x84, 0x00]).unwrap();
        assert_eq!(read_dpcd(&device, 1, 3), Some(vec![0x14, 0x1e, 0x84]));
        assert_eq!(read_dpcd(&device, 3, 8), None);
    }
}
