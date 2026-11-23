/// Which advanced controls the stack on this machine can drive.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct BluetoothCapabilities {
    pub codec_switch: bool,
    pub pan_tethering: bool,
    pub auracast: bool,
    pub avrcp_config: bool,
    pub sample_rate: bool,
    pub a2dp_offload: bool,
    pub hci_snoop: bool,
    pub gatt_cache_clear: bool,
}
