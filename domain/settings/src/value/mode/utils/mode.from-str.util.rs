use std::str::FromStr;

use crate::error::DomainError;
use crate::value::mode::interfaces::Mode;
use crate::value::{RefreshCentiHz, Resolution};

/// Accepts `WxH@R`, `WxH@RHz` (both Hyprland and kanshi spellings).
impl FromStr for Mode {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || DomainError::InvalidMode(s.to_owned());
        let (res, rate) = s.split_once('@').ok_or_else(invalid)?;
        let rate = rate
            .strip_suffix("Hz")
            .or_else(|| rate.strip_suffix("hz"))
            .unwrap_or(rate);
        Ok(Self {
            resolution: Resolution::parse(res).ok_or_else(invalid)?,
            refresh: RefreshCentiHz::parse(rate).ok_or_else(invalid)?,
        })
    }
}
