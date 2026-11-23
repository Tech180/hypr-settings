use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::night_light::KELVIN_DEFAULT;

/// Same JSON keys as the AGS preferences file this replaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DisplayPreferences {
    /// Connector hosting the main bar, if the user pinned one.
    pub bar_host_connector: Option<String>,
    /// Extra refresh rates per connector; only listed when advertised.
    pub custom_hz: BTreeMap<String, Vec<u32>>,
    /// The brightness slider drives every controllable display at once.
    pub sync_brightness: bool,
    /// Manual night light, which suspends the hyprsunset schedule.
    pub night_override: bool,
    pub night_kelvin: u32,
}

impl Default for DisplayPreferences {
    fn default() -> Self {
        Self {
            bar_host_connector: None,
            custom_hz: BTreeMap::new(),
            sync_brightness: true,
            night_override: false,
            night_kelvin: KELVIN_DEFAULT,
        }
    }
}
