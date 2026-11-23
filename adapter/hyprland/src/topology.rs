//! `hyprctl -j monitors all` -> domain `Topology`. All Hyprland wire formats
//! (`1920x1080@143.97Hz`, float refresh, transform ints) are parsed here.

use std::sync::Arc;

use serde::Deserialize;
use settings_domain::ports::TopologyReader;
use settings_domain::{
    AdvertisedModes, ConnectorName, DisplayKind, Mode, Output, PortError, Position, RefreshCentiHz,
    Resolution, Scale, Topology, Transform,
};
use thiserror::Error;

use crate::exec::{Exec, SystemExec};

#[derive(Debug, Error)]
pub enum TopologyParseError {
    #[error("hyprctl returned invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RawMonitor {
    name: String,
    description: String,
    make: String,
    model: String,
    serial: String,
    width: u32,
    height: u32,
    refresh_rate: f64,
    x: i32,
    y: i32,
    scale: f64,
    transform: i32,
    focused: bool,
    disabled: bool,
    current_format: String,
    color_management_preset: String,
    available_modes: Vec<String>,
    vrr: bool,
}

fn is_ten_bit(format: &str) -> bool {
    let f = format.to_uppercase();
    ["2101010", "101010", "P010", "AR30", "AB30"]
        .iter()
        .any(|needle| f.contains(needle))
}

impl RawMonitor {
    fn into_output(self) -> Option<Output> {
        let connector = ConnectorName::try_from(self.name.as_str()).ok()?;
        let description = if self.description.trim().is_empty() {
            let joined = format!("{} {}", self.make, self.model);
            if joined.trim().is_empty() {
                connector.to_string()
            } else {
                joined.trim().to_owned()
            }
        } else {
            self.description.trim().to_owned()
        };

        let resolution = Resolution::new(self.width, self.height);
        let mode = (!resolution.is_empty())
            .then(|| Mode::new(resolution, RefreshCentiHz::from_hz(self.refresh_rate)));

        Some(Output {
            kind: DisplayKind::infer(&connector, &description),
            connector,
            description,
            make: self.make,
            model: self.model,
            serial: self.serial,
            mode,
            position: Position::new(self.x, self.y),
            scale: if self.scale > 0.0 {
                Scale::clamped(self.scale)
            } else {
                Scale::clamped(1.0)
            },
            transform: Transform::from_hypr(self.transform),
            enabled: !self.disabled,
            focused: self.focused && !self.disabled,
            modes: self
                .available_modes
                .iter()
                .filter_map(|m| m.parse::<Mode>().ok())
                .collect::<AdvertisedModes>(),
            vrr: self.vrr,
            hdr: self.color_management_preset.to_lowercase().contains("hdr"),
            ten_bit: is_ten_bit(&self.current_format),
        })
    }
}

pub fn parse_monitors(json: &str) -> Result<Topology, TopologyParseError> {
    let raw: Vec<RawMonitor> = serde_json::from_str(json)?;
    Ok(Topology::new(
        raw.into_iter()
            .filter_map(RawMonitor::into_output)
            .collect(),
    ))
}

pub struct HyprctlTopology {
    exec: Arc<dyn Exec>,
}

impl HyprctlTopology {
    pub fn new() -> Self {
        Self::with_exec(Arc::new(SystemExec))
    }

    pub fn with_exec(exec: Arc<dyn Exec>) -> Self {
        Self { exec }
    }
}

impl Default for HyprctlTopology {
    fn default() -> Self {
        Self::new()
    }
}

impl TopologyReader for HyprctlTopology {
    fn read(&self) -> Result<Topology, PortError> {
        // `all` includes outputs the compositor disabled, so they can be re-enabled.
        let json = self
            .exec
            .run("hyprctl", &["-j", "monitors", "all"])
            .or_else(|_| self.exec.run("hyprctl", &["-j", "monitors"]))
            .map_err(|e| PortError::Unavailable(e.to_string()))?;
        parse_monitors(&json).map_err(|e| PortError::Invalid(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::ExecError;

    const FIXTURE: &str = include_str!("../tests/fixtures/monitors-all.json");

    fn conn(name: &str) -> ConnectorName {
        ConnectorName::try_from(name).unwrap()
    }

    #[test]
    fn parses_a_real_capture() {
        let topology = parse_monitors(FIXTURE).unwrap();
        assert_eq!(topology.outputs().len(), 3);
        assert_eq!(topology.enabled_count(), 2);

        let laptop = topology.find(&conn("eDP-1")).unwrap();
        assert_eq!(laptop.kind, DisplayKind::Laptop);
        assert_eq!(laptop.mode.unwrap().refresh.rounded_hz(), 60);
        assert_eq!(laptop.description, "BOE 0x0A89");

        let wide = topology.find(&conn("DP-3")).unwrap();
        assert!(wide.is_ultrawide());
        assert!(wide.focused);
        assert_eq!(wide.position, Position::new(1920, 0));
        assert!(wide.modes.supports(Resolution::new(2560, 1080), 100));
    }

    #[test]
    fn keeps_fractional_advertised_rates() {
        let topology = parse_monitors(FIXTURE).unwrap();
        let wide = topology.find(&conn("DP-3")).unwrap();
        let exact = wide
            .modes
            .exact_for(Resolution::new(1920, 1080), 120)
            .unwrap();
        assert_eq!(exact.refresh, RefreshCentiHz::from_hz(120.0));
        assert!(
            wide.modes
                .iter()
                .any(|m| m.refresh == RefreshCentiHz::from_hz(119.88))
        );
    }

    #[test]
    fn disabled_output_has_no_mode_but_keeps_its_modes() {
        let topology = parse_monitors(FIXTURE).unwrap();
        let off = topology.find(&conn("HDMI-A-1")).unwrap();
        assert!(!off.enabled);
        assert!(!off.focused);
        assert_eq!(off.mode, None);
        assert!(!off.modes.is_empty());
    }

    #[test]
    fn reads_transform_hdr_and_bit_depth() {
        let topology = parse_monitors(FIXTURE).unwrap();
        let off = topology.find(&conn("HDMI-A-1")).unwrap();
        assert_eq!(off.transform, Transform::R90);
        assert!(off.hdr);
        assert!(off.ten_bit);
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(parse_monitors("{").is_err());
    }

    struct Scripted(Vec<(&'static str, Result<String, String>)>);

    impl Exec for Scripted {
        fn run(&self, _program: &str, args: &[&str]) -> Result<String, ExecError> {
            let key = args.join(" ");
            self.0
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.clone().map_err(ExecError))
                .unwrap_or_else(|| Err(ExecError(format!("unexpected {key}"))))
        }
    }

    #[test]
    fn falls_back_to_plain_monitors_listing() {
        let exec = Scripted(vec![
            ("-j monitors all", Err("old hyprland".into())),
            ("-j monitors", Ok(FIXTURE.to_owned())),
        ]);
        let reader = HyprctlTopology::with_exec(Arc::new(exec));
        assert_eq!(reader.read().unwrap().outputs().len(), 3);
    }

    #[test]
    fn reports_unavailable_when_hyprctl_fails() {
        let reader = HyprctlTopology::with_exec(Arc::new(Scripted(Vec::new())));
        assert!(matches!(reader.read(), Err(PortError::Unavailable(_))));
    }
}
