use std::collections::BTreeMap;

use super::device_class::DeviceClass;
use super::device_id::DeviceId;

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct BluetoothPreferences {
    #[serde(default)]
    pub device_class: BTreeMap<String, DeviceClass>,
    #[serde(default)]
    pub developer: DeveloperPreferences,
    /// MAC address of the adapter the user chose; absent means use the first BlueZ lists.
    #[serde(default)]
    pub selected_adapter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct DeveloperPreferences {
    #[serde(default)]
    pub avrcp_version: Option<String>,
    #[serde(default)]
    pub sample_rate_hz: Option<u32>,
    #[serde(default)]
    pub a2dp_offload_disabled: bool,
    #[serde(default)]
    pub hci_snoop: bool,
    #[serde(default)]
    pub pan_enabled: bool,
}

impl BluetoothPreferences {
    pub fn class_for(&self, id: &DeviceId, fallback: DeviceClass) -> DeviceClass {
        self.device_class
            .get(id.as_str())
            .copied()
            .unwrap_or(fallback)
    }

    pub fn set_class(&mut self, id: &DeviceId, class: DeviceClass) {
        self.device_class.insert(id.0.clone(), class);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_json_without_selected_adapter_loads() {
        let json = r#"{"device_class":{},"developer":{}}"#;
        let prefs: BluetoothPreferences = serde_json::from_str(json).unwrap();
        assert!(prefs.selected_adapter.is_none());
    }
}
