use super::constants::{KELVIN_DEFAULT, KELVIN_MAX, KELVIN_MIN};
use super::interfaces::Kelvin;

impl Kelvin {
    pub fn clamped(value: u32) -> Self {
        Self(value.clamp(KELVIN_MIN, KELVIN_MAX))
    }

    pub fn get(self) -> u32 {
        self.0
    }

    /// 0.0 at the warmest end, 1.0 at the coolest.
    pub fn fraction(self) -> f64 {
        f64::from(self.0 - KELVIN_MIN) / f64::from(KELVIN_MAX - KELVIN_MIN)
    }

    pub fn from_fraction(fraction: f64) -> Self {
        let span = f64::from(KELVIN_MAX - KELVIN_MIN);
        Self::clamped(KELVIN_MIN + (fraction.clamp(0.0, 1.0) * span).round() as u32)
    }
}

impl Default for Kelvin {
    fn default() -> Self {
        Self(KELVIN_DEFAULT)
    }
}
