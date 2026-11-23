use crate::error::DomainError;
use crate::value::Resolution;

use super::constants::{ALL_VALID_MAX_HUNDREDTHS, COMMON_SCALES, MAX_HUNDREDTHS, MIN_HUNDREDTHS};
use super::interfaces::Scale;

impl Scale {
    /// Known-valid hundredths for constants and presets (not for arbitrary input).
    pub(crate) const fn raw(hundredths: u16) -> Self {
        Self(hundredths)
    }

    pub fn new(value: f64) -> Result<Self, DomainError> {
        if !value.is_finite() {
            return Err(DomainError::InvalidScale(value));
        }
        let hundredths = (value * 100.0).round();
        if hundredths < f64::from(MIN_HUNDREDTHS) || hundredths > f64::from(MAX_HUNDREDTHS) {
            return Err(DomainError::InvalidScale(value));
        }
        Ok(Self(hundredths as u16))
    }

    /// Lenient constructor for values read from the compositor.
    pub fn clamped(value: f64) -> Self {
        if value.is_finite() {
            Self(
                ((value * 100.0).round() as i64).clamp(MIN_HUNDREDTHS.into(), MAX_HUNDREDTHS.into())
                    as u16,
            )
        } else {
            Self(100)
        }
    }

    pub fn as_f64(self) -> f64 {
        f64::from(self.0) / 100.0
    }

    pub fn hundredths(self) -> u16 {
        self.0
    }

    /// Chip label such as `1×` or `1.25×`.
    pub fn label(self) -> String {
        format!("{self}×")
    }

    /// Hyprland only keeps a scale when it divides the mode into whole logical
    /// pixels; otherwise it silently swaps in another one.
    pub fn is_valid_for(self, resolution: Resolution) -> bool {
        let s = u64::from(self.0);
        s != 0
            && (u64::from(resolution.width) * 100).is_multiple_of(s)
            && (u64::from(resolution.height) * 100).is_multiple_of(s)
    }

    /// Every scale between 0.5 and 3.0 that `is_valid_for` the resolution.
    pub fn all_valid_for(resolution: Resolution) -> Vec<Self> {
        (MIN_HUNDREDTHS..=ALL_VALID_MAX_HUNDREDTHS)
            .map(Self)
            .filter(|s| s.is_valid_for(resolution))
            .collect()
    }

    /// The valid scale closest to `self` (the lower one on a tie); 1.0 always
    /// divides, so there is always an answer.
    pub fn nearest_valid_for(self, resolution: Resolution) -> Self {
        if self.is_valid_for(resolution) {
            return self;
        }
        (MIN_HUNDREDTHS..=MAX_HUNDREDTHS)
            .map(Self)
            .filter(|s| s.is_valid_for(resolution))
            .min_by_key(|s| s.0.abs_diff(self.0))
            .unwrap_or(Self(100))
    }

    /// Scales worth offering in the UI: the common steps that are valid for the
    /// resolution, plus the current one so the active state is always visible.
    pub fn offered_for(resolution: Resolution, current: Self) -> Vec<Self> {
        let mut offered: Vec<Self> = COMMON_SCALES
            .into_iter()
            .map(Self)
            .filter(|s| s.is_valid_for(resolution))
            .collect();
        if !offered.contains(&current) {
            offered.push(current);
            offered.sort();
        }
        if offered.is_empty() {
            offered.push(Self(100));
        }
        offered
    }
}
