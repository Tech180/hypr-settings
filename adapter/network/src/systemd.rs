use std::sync::Arc;

use settings_domain::PortError;
use settings_domain::ports::SystemdUnits;

use crate::exec::Exec;
use crate::nm::port;

const UNIT_FILE_OK: [&str; 4] = ["linked", "enabled", "disabled", "static"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgUnit {
    pub unit: String,
    pub iface: String,
    pub active: bool,
}

/// `systemctl` for the NixOS `wg-quick-*.service` tunnels.
#[derive(Clone)]
pub struct Systemctl {
    exec: Arc<dyn Exec>,
}

impl Systemctl {
    pub fn new(exec: Arc<dyn Exec>) -> Self {
        Self { exec }
    }

    fn run(&self, args: &[&str]) -> Result<String, crate::exec::ExecError> {
        self.exec.run("systemctl", args)
    }

    fn property(&self, unit: &str, name: &str) -> Option<String> {
        self.run(&["show", unit, "-p", name, "--value"])
            .ok()
            .map(|s| s.trim().to_owned())
    }

    pub fn is_active(&self, unit: &str) -> bool {
        self.run(&["is-active", unit])
            .is_ok_and(|s| matches!(s.trim(), "active" | "activating"))
    }

    /// `wg0` → `wg-quick-wg0.service` when that unit exists.
    pub fn resolve_wg_unit(&self, iface: &str) -> Option<String> {
        if iface.is_empty() {
            return None;
        }
        let unit = format!("wg-quick-{iface}.service");
        let loaded = self.property(&unit, "LoadState").as_deref() == Some("loaded");
        let file_ok = self
            .property(&unit, "UnitFileState")
            .is_some_and(|s| UNIT_FILE_OK.contains(&s.as_str()));
        (loaded || file_ok).then_some(unit)
    }

    /// Installed `wg-quick-*.service` units, masked ones skipped.
    pub fn list_wg_units(&self) -> Vec<WgUnit> {
        let Ok(out) = self.run(&[
            "list-unit-files",
            "wg-quick-*.service",
            "--no-legend",
            "--plain",
        ]) else {
            return Vec::new();
        };
        parse_unit_files(&out)
            .into_iter()
            .map(|(unit, iface)| WgUnit {
                active: self.is_active(&unit),
                unit,
                iface,
            })
            .collect()
    }
}

/// `(unit, iface)` of each non-masked `wg-quick-<iface>.service` line.
pub fn parse_unit_files(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let unit = parts.next()?;
            if parts.next() == Some("masked") {
                return None;
            }
            let iface = unit
                .strip_prefix("wg-quick-")?
                .strip_suffix(".service")?
                .to_owned();
            Some((unit.to_owned(), iface))
        })
        .collect()
}

impl SystemdUnits for Systemctl {
    fn start(&self, unit: &str) -> Result<(), PortError> {
        self.run(&["start", unit]).map(drop).map_err(port)
    }

    fn stop(&self, unit: &str) -> Result<(), PortError> {
        self.run(&["stop", unit]).map(drop).map_err(port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_files_skip_masked_and_foreign_units() {
        let units = parse_unit_files(
            "wg-quick-mullvad.service enabled enabled\nwg-quick-old.service masked enabled\nother.service enabled enabled\n\n",
        );
        assert_eq!(
            units,
            [("wg-quick-mullvad.service".into(), "mullvad".into())]
        );
    }
}
