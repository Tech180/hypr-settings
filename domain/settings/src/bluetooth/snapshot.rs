use super::device_class::DeviceClass;
use super::device_id::DeviceId;
use super::profile::ProfileState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterSnapshot {
    pub name: String,
    pub address: String,
    pub alias: String,
    pub powered: bool,
    pub discoverable: bool,
    pub discovering: bool,
    pub lmp_version: Option<String>,
    pub modalias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileSnapshot {
    pub media_audio: ProfileState,
    pub phone_calls: ProfileState,
    pub contact_sharing: ProfileState,
    pub absolute_volume: ProfileState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecSnapshot {
    pub active: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSnapshot {
    pub id: DeviceId,
    pub name: String,
    pub address: String,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    pub battery_percent: Option<u8>,
    pub uuids: Vec<String>,
    pub class: DeviceClass,
    pub profiles: ProfileSnapshot,
    pub codec: Option<CodecSnapshot>,
}

impl DeviceSnapshot {
    pub fn shows_codec_row(&self) -> bool {
        self.class == DeviceClass::Headphones && self.codec.is_some()
    }
}
