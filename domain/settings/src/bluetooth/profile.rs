/// Bluetooth profile exposed in the device detail page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    MediaAudio,
    PhoneCalls,
    ContactSharing,
    AbsoluteVolume,
}

impl ProfileKind {
    pub const DETAIL_ROWS: [ProfileKind; 4] = [
        ProfileKind::MediaAudio,
        ProfileKind::PhoneCalls,
        ProfileKind::AbsoluteVolume,
        ProfileKind::ContactSharing,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileState {
    pub available: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecId(pub String);

impl CodecId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}
