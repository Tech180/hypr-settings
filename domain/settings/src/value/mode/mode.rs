use super::interfaces::Mode;
use crate::value::{RefreshCentiHz, Resolution};

impl Mode {
    pub fn new(resolution: Resolution, refresh: RefreshCentiHz) -> Self {
        Self {
            resolution,
            refresh,
        }
    }

    /// `1920x1080@60` or `2560x1080@164.99`.
    pub fn kanshi_token(self) -> String {
        self.to_string()
    }

    /// `1920x1080@60.00`, the form `hl.monitor` expects.
    pub fn hypr_token(self) -> String {
        format!("{}@{}", self.resolution, self.refresh.fixed2())
    }
}
