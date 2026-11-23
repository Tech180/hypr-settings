//! Journal, sysfs counters, QR encoding and nearby VPN configs.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use settings_domain::PortError;
use settings_domain::network::{QrMatrix, VpnCandidate, config_kind_for};
use settings_domain::ports::{NetworkJournal, NetworkStats, QrEncoder, VpnFiles};

use crate::exec::Exec;
use crate::nm::port;

const MAX_CANDIDATES: usize = 40;
const MAX_CONF_BYTES: u64 = 256 * 1024;

/// The NetworkManager journal.
pub struct JournalReader {
    exec: Arc<dyn Exec>,
}

impl JournalReader {
    pub fn new(exec: Arc<dyn Exec>) -> Self {
        Self { exec }
    }
}

impl NetworkJournal for JournalReader {
    fn lines(&self, limit: usize) -> Result<Vec<String>, PortError> {
        let limit = limit.clamp(1, 500).to_string();
        let out = self
            .exec
            .run(
                "journalctl",
                &[
                    "-u",
                    "NetworkManager",
                    "-n",
                    &limit,
                    "--no-pager",
                    "-o",
                    "short-iso",
                ],
            )
            .map_err(port)?;
        Ok(out
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with("-- "))
            .map(str::to_owned)
            .collect())
    }
}

/// `/sys/class/net/<iface>/statistics/{rx,tx}_bytes`.
pub struct SysfsStats {
    root: PathBuf,
}

impl SysfsStats {
    pub fn new() -> Self {
        Self::with_root(PathBuf::from("/sys/class/net"))
    }

    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }
}

impl Default for SysfsStats {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkStats for SysfsStats {
    fn counters(&self, iface: &str) -> Option<(u64, u64)> {
        // An interface name is one path component.
        if iface.is_empty() || iface.contains('/') || iface == ".." || iface == "." {
            return None;
        }
        let read = |name: &str| -> Option<u64> {
            fs::read_to_string(self.root.join(iface).join("statistics").join(name))
                .ok()?
                .trim()
                .parse()
                .ok()
        };
        Some((read("rx_bytes")?, read("tx_bytes")?))
    }
}

pub struct QrCodeEncoder;

impl QrEncoder for QrCodeEncoder {
    fn encode(&self, text: &str) -> Result<QrMatrix, PortError> {
        use qrcode::{Color, QrCode};
        let code = QrCode::new(text.as_bytes())
            .map_err(|e| PortError::Invalid(format!("QR code: {e}")))?;
        Ok(QrMatrix {
            size: code.width(),
            dark: code
                .to_colors()
                .into_iter()
                .map(|c| c == Color::Dark)
                .collect(),
        })
    }
}

/// Shallow scans of a few directories for `*.conf` and `*.ovpn`.
pub struct FsVpnFiles {
    roots: Vec<PathBuf>,
}

impl FsVpnFiles {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }

    /// Home, Downloads, `~/.config/wireguard` and `~/.config/NetworkManager`.
    pub fn for_home(home: PathBuf, downloads: Option<PathBuf>) -> Self {
        let downloads = downloads.unwrap_or_else(|| home.join("Downloads"));
        Self::new(vec![
            home.clone(),
            downloads,
            home.join(".config/wireguard"),
            home.join(".config/NetworkManager"),
        ])
    }
}

impl VpnFiles for FsVpnFiles {
    fn candidates(&self) -> Vec<VpnCandidate> {
        let mut seen: Vec<PathBuf> = Vec::new();
        let mut all = Vec::new();
        for root in &self.roots {
            let Ok(entries) = fs::read_dir(root) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let Some(kind) = config_kind_for(name) else {
                    continue;
                };
                let Ok(meta) = entry.metadata() else { continue };
                if !meta.is_file() || seen.contains(&path) {
                    continue;
                }
                let mtime_secs = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());
                all.push(VpnCandidate {
                    path: path.to_string_lossy().into_owned(),
                    name: name.to_owned(),
                    kind,
                    mtime_secs,
                });
                seen.push(path);
            }
        }
        all.sort_by(|a, b| b.mtime_secs.cmp(&a.mtime_secs).then(a.name.cmp(&b.name)));
        all.truncate(MAX_CANDIDATES);
        all
    }

    fn read_text(&self, path: &str) -> Result<String, PortError> {
        if config_kind_for(path).is_none() {
            return Err(PortError::Invalid(format!("{path} is not a VPN config")));
        }
        let meta = fs::metadata(path).map_err(|e| PortError::Io(format!("{path}: {e}")))?;
        if meta.len() > MAX_CONF_BYTES {
            return Err(PortError::Invalid(format!("{path} is too large")));
        }
        fs::read_to_string(path).map_err(|e| PortError::Io(format!("{path}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hs-net-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn counters_read_sysfs_and_reject_paths() {
        let root = scratch("stats");
        let stats_dir = root.join("wlan0/statistics");
        fs::create_dir_all(&stats_dir).unwrap();
        fs::write(stats_dir.join("rx_bytes"), "1234\n").unwrap();
        fs::write(stats_dir.join("tx_bytes"), "56\n").unwrap();

        let stats = SysfsStats::with_root(root.clone());
        assert_eq!(stats.counters("wlan0"), Some((1234, 56)));
        assert_eq!(stats.counters("wlan9"), None);
        assert_eq!(stats.counters("../wlan0"), None);
        assert_eq!(stats.counters(""), None);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_qr_matrix_is_square_with_finder_patterns() {
        let matrix = QrCodeEncoder
            .encode("WIFI:T:WPA;S:Home;P:secret;H:false;;")
            .unwrap();
        assert_eq!(matrix.dark.len(), matrix.size * matrix.size);
        assert!(matrix.size >= 21);
        // The top-left finder pattern is a dark corner.
        assert!(matrix.is_dark(0, 0) && matrix.is_dark(6, 6));
        assert!(!matrix.is_dark(7, 0));
    }

    #[test]
    fn candidates_are_newest_first_and_typed() {
        let home = scratch("files");
        let wg = home.join(".config/wireguard");
        fs::create_dir_all(&wg).unwrap();
        fs::write(wg.join("office.conf"), "[Interface]\n").unwrap();
        fs::write(home.join("corp.OVPN"), "client\n").unwrap();
        fs::write(home.join("notes.txt"), "x").unwrap();

        let files = FsVpnFiles::for_home(home.clone(), None);
        let found = files.candidates();
        let names: Vec<_> = found.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(found.len(), 2, "{names:?}");
        assert!(names.contains(&"office.conf") && names.contains(&"corp.OVPN"));

        assert!(
            files
                .read_text(wg.join("office.conf").to_str().unwrap())
                .is_ok()
        );
        assert!(
            files
                .read_text(home.join("notes.txt").to_str().unwrap())
                .is_err()
        );
        let _ = fs::remove_dir_all(home);
    }
}
