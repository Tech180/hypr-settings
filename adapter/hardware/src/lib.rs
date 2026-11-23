//! Driven adapter for display hardware: DDC/CI through `ddcutil`, the laptop backlight
//! through `brightnessctl` and the night light through `hyprsunset`.

mod backlight;
mod ddc;
mod exec;
mod night_light;

pub use backlight::BrightnessctlBacklight;
pub use ddc::DdcutilDdc;
pub use exec::{Exec, ExecError, SystemExec};
pub use night_light::HyprsunsetNightLight;
