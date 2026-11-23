/// MCCS feature codes (ddcutil takes them in hexadecimal).
pub const VCP_BRIGHTNESS: u8 = 0x10;
pub const VCP_CONTRAST: u8 = 0x12;
pub const VCP_INPUT_SOURCE: u8 = 0x60;

/// Longest custom refresh rate that is accepted.
pub const MAX_CUSTOM_HZ: u32 = 1000;

/// Brightness change of one bar scroll tick or key press.
pub const BRIGHTNESS_STEP: i32 = 5;
