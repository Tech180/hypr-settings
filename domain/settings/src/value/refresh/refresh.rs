use super::interfaces::RefreshCentiHz;

impl RefreshCentiHz {
    pub fn from_hz(hz: f64) -> Self {
        if hz.is_finite() && hz > 0.0 {
            Self((hz * 100.0).round() as u32)
        } else {
            Self(0)
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        let hz: f64 = text.trim().parse().ok()?;
        (hz.is_finite() && hz > 0.0).then(|| Self::from_hz(hz))
    }

    pub fn rounded_hz(self) -> u32 {
        (self.0 + 50) / 100
    }

    /// Two-decimal form used for Lua `hl.monitor` (`143.97`, `60.00`).
    pub fn fixed2(self) -> String {
        format!("{}.{:02}", self.0 / 100, self.0 % 100)
    }
}
