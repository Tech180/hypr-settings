use super::subscription::Subscription;
use crate::bluetooth::{
    AdapterSnapshot, BluetoothCapabilities, BluetoothPreferences, CodecId, DeviceId,
    DeviceSnapshot, ProfileKind,
};
use crate::error::PortError;

pub trait BluetoothGateway: Send + Sync {
    fn adapters(&self) -> Result<Vec<AdapterSnapshot>, PortError>;
    /// `address` is a MAC; `None` means the first adapter BlueZ lists.
    fn set_active_adapter(&self, address: Option<&str>) -> Result<(), PortError>;
    fn adapter(&self) -> Result<Option<AdapterSnapshot>, PortError>;
    fn devices(&self) -> Result<Vec<DeviceSnapshot>, PortError>;
    fn set_powered(&self, on: bool) -> Result<(), PortError>;
    fn set_discoverable(&self, on: bool) -> Result<(), PortError>;
    fn start_discovery(&self) -> Result<(), PortError>;
    fn stop_discovery(&self) -> Result<(), PortError>;
    fn pair(&self, id: &DeviceId) -> Result<(), PortError>;
    fn trust(&self, id: &DeviceId, trust: bool) -> Result<(), PortError>;
    fn connect(&self, id: &DeviceId) -> Result<(), PortError>;
    fn disconnect(&self, id: &DeviceId) -> Result<(), PortError>;
    fn remove(&self, id: &DeviceId) -> Result<(), PortError>;
    fn set_profile(
        &self,
        id: &DeviceId,
        profile: ProfileKind,
        enable: bool,
    ) -> Result<(), PortError>;
    fn set_codec(&self, id: &DeviceId, codec: &CodecId) -> Result<(), PortError>;
    fn set_absolute_volume(&self, id: &DeviceId, on: bool) -> Result<(), PortError>;
    fn capabilities(&self) -> BluetoothCapabilities;
}

pub trait BluetoothEvents: Send + Sync {
    fn subscribe(&self, sink: Box<dyn Fn() + Send>) -> Result<Subscription, PortError>;
}

pub trait BluetoothExtras: Send + Sync {
    fn pan_enabled(&self) -> Result<bool, PortError>;
    fn set_pan_enabled(&self, on: bool) -> Result<(), PortError>;
    fn avrcp_versions(&self) -> Result<Vec<String>, PortError>;
    fn avrcp_version(&self) -> Result<Option<String>, PortError>;
    fn set_avrcp_version(&self, version: &str) -> Result<(), PortError>;
    fn sample_rates(&self) -> Result<Vec<u32>, PortError>;
    fn sample_rate_hz(&self) -> Result<Option<u32>, PortError>;
    fn set_sample_rate_hz(&self, hz: u32) -> Result<(), PortError>;
    fn a2dp_offload_disabled(&self) -> Result<bool, PortError>;
    fn set_a2dp_offload_disabled(&self, disabled: bool) -> Result<(), PortError>;
    fn hci_snoop_enabled(&self) -> Result<bool, PortError>;
    fn set_hci_snoop_enabled(&self, on: bool) -> Result<(), PortError>;
    fn clear_gatt_cache(&self) -> Result<(), PortError>;
    fn auracast_available(&self) -> bool;
}

pub trait BluetoothPreferencesStore: Send + Sync {
    fn load(&self) -> Result<BluetoothPreferences, PortError>;
    fn save(&self, prefs: &BluetoothPreferences) -> Result<(), PortError>;
}
