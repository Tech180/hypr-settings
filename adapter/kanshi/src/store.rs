use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use settings_domain::ports::ProfileStore;
use settings_domain::{PortError, ProfileSet};

use crate::format;

const STUB: &str = "profile default {\n  output \"*\" scale 1\n}\n";

/// Writes next to `path`, fsyncs, then renames, so readers never see a partial file.
pub(crate) fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(dir) = dir {
        fs::create_dir_all(dir)?;
    }
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?
        .to_string_lossy();
    let tmp = path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));

    let result = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn io_err(path: &Path, err: io::Error) -> PortError {
    PortError::Io(format!("{}: {err}", path.display()))
}

/// The runtime kanshi.conf that kanshi is launched against.
pub struct KanshiConfFile {
    path: PathBuf,
    /// Tried in order to seed a missing conf; a stub is written when none exist.
    seeds: Vec<PathBuf>,
}

impl KanshiConfFile {
    pub fn new(path: PathBuf, seeds: Vec<PathBuf>) -> Self {
        Self { path, seeds }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn bootstrap(&self) -> Result<(), PortError> {
        let text = match self.seeds.iter().find_map(|p| fs::read_to_string(p).ok()) {
            Some(text) => {
                tracing::info!(path = %self.path.display(), "seeding kanshi conf");
                text
            }
            None => {
                tracing::warn!(path = %self.path.display(), "no seed found; writing stub kanshi conf");
                STUB.to_owned()
            }
        };
        write_atomic(&self.path, &text).map_err(|e| io_err(&self.path, e))
    }
}

impl ProfileStore for KanshiConfFile {
    fn load(&self) -> Result<ProfileSet, PortError> {
        if !self.path.exists() {
            self.bootstrap()?;
        }
        let text = fs::read_to_string(&self.path).map_err(|e| io_err(&self.path, e))?;
        Ok(format::parse(&text))
    }

    fn save(&self, set: &ProfileSet) -> Result<(), PortError> {
        write_atomic(&self.path, &format::serialize(set)).map_err(|e| io_err(&self.path, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hypr-settings-kanshi-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn bootstraps_from_the_first_existing_seed() {
        let dir = temp_dir("seed");
        let seed = dir.join("hm-config");
        fs::write(&seed, "profile portable {\n  output \"eDP-1\" enable\n}\n").unwrap();
        let store = KanshiConfFile::new(
            dir.join("state/hypr-settings/kanshi.conf"),
            vec![dir.join("missing"), seed],
        );

        let set = store.load().unwrap();
        assert_eq!(set.profiles[0].name.as_str(), "portable");
        assert!(store.path().exists());
    }

    #[test]
    fn writes_a_stub_when_no_seed_exists() {
        let dir = temp_dir("stub");
        let store = KanshiConfFile::new(dir.join("kanshi.conf"), vec![dir.join("nope")]);
        let set = store.load().unwrap();
        assert_eq!(set.profiles.len(), 1);
        assert!(set.profiles[0].name.is_default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = temp_dir("roundtrip");
        let store = KanshiConfFile::new(dir.join("kanshi.conf"), Vec::new());
        let set = format::parse(include_str!("../tests/fixtures/kanshi-seed.conf"));
        store.save(&set).unwrap();
        assert_eq!(store.load().unwrap(), set);
    }

    #[test]
    fn failed_write_leaves_the_target_and_no_temp_files() {
        let dir = temp_dir("atomic");
        // A directory cannot be replaced by a file, so the rename step fails.
        let target = dir.join("kanshi.conf");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("keep"), "x").unwrap();

        assert!(write_atomic(&target, "profile a {\n}\n").is_err());
        assert!(target.join("keep").exists());
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }
}
