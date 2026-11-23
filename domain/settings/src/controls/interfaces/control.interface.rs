#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ControlKind {
    #[default]
    None,
    /// The laptop panel's backlight.
    Internal,
    /// DDC/CI over the display cable.
    Ddc,
}

/// One 0-100 control (brightness, contrast, overdrive) and why it may be unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Control {
    pub kind: ControlKind,
    pub percent: Option<u8>,
    pub locked: bool,
    pub reason: String,
    /// The VCP feature code, for controls discovered at runtime (overdrive).
    pub code: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSource {
    pub label: String,
    /// The VCP 0x60 value.
    pub code: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputSources {
    pub current: Option<InputSource>,
    pub options: Vec<InputSource>,
}

/// One display `ddcutil detect` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdcDisplay {
    pub bus: u32,
    pub connector: Option<String>,
    pub model: String,
    pub serial: String,
}
