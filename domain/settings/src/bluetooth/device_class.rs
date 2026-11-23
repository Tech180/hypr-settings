/// UI device category shown in lists and the detail type dropdown.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum DeviceClass {
    Headphones,
    Keyboard,
    Mouse,
    Phone,
    Gamepad,
    #[default]
    Other,
}

impl DeviceClass {
    pub const ALL: [DeviceClass; 6] = [
        DeviceClass::Headphones,
        DeviceClass::Keyboard,
        DeviceClass::Mouse,
        DeviceClass::Phone,
        DeviceClass::Gamepad,
        DeviceClass::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Headphones => "Headphones",
            Self::Keyboard => "Keyboard",
            Self::Mouse => "Mouse",
            Self::Phone => "Phone",
            Self::Gamepad => "Gamepad",
            Self::Other => "Other",
        }
    }
}
