use std::collections::BTreeSet;

use crate::modes::interfaces::AdvertisedModes;
use crate::value::{Mode, Resolution};

impl AdvertisedModes {
    pub fn new(modes: Vec<Mode>) -> Self {
        Self(modes)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Mode> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Unique resolutions, highest pixel count first.
    pub fn resolutions(&self) -> Vec<Resolution> {
        let mut seen = Vec::new();
        for mode in &self.0 {
            if !seen.contains(&mode.resolution) {
                seen.push(mode.resolution);
            }
        }
        seen.sort_by_key(|r| std::cmp::Reverse(r.pixels()));
        seen
    }

    /// Rounded rates for a resolution, plus the current rate. A custom rate is
    /// only listed when it is actually advertised.
    pub fn hz_options(&self, res: Resolution, current_hz: u32, custom: &[u32]) -> Vec<u32> {
        let mut rates: BTreeSet<u32> = self
            .0
            .iter()
            .filter(|m| m.resolution == res)
            .map(|m| m.refresh.rounded_hz())
            .filter(|hz| *hz > 0)
            .collect();
        if current_hz > 0 {
            rates.insert(current_hz);
        }
        rates.extend(custom.iter().copied().filter(|hz| self.supports(res, *hz)));
        rates.into_iter().collect()
    }

    /// The advertised mode whose rounded rate matches, e.g. chip 144 gives `143.97`.
    pub fn exact_for(&self, res: Resolution, rounded_hz: u32) -> Option<Mode> {
        self.0
            .iter()
            .find(|m| m.resolution == res && m.refresh.rounded_hz() == rounded_hz)
            .copied()
    }

    pub fn supports(&self, res: Resolution, rounded_hz: u32) -> bool {
        self.exact_for(res, rounded_hz).is_some()
    }

    pub fn has_resolution(&self, res: Resolution) -> bool {
        self.0.iter().any(|m| m.resolution == res)
    }

    /// Keep the preferred rate when supported, otherwise the highest advertised.
    pub fn preferred_for_resolution(&self, res: Resolution, prefer_hz: u32) -> Option<u32> {
        if self.supports(res, prefer_hz) {
            return Some(prefer_hz);
        }
        self.0
            .iter()
            .filter(|m| m.resolution == res)
            .map(|m| m.refresh.rounded_hz())
            .filter(|hz| *hz > 0)
            .max()
    }

    pub fn token_for_resolution(&self, res: Resolution, prefer_hz: u32) -> Option<Mode> {
        let hz = self.preferred_for_resolution(res, prefer_hz)?;
        self.exact_for(res, hz)
    }
}

impl FromIterator<Mode> for AdvertisedModes {
    fn from_iter<T: IntoIterator<Item = Mode>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
