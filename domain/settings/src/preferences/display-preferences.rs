use super::interfaces::DisplayPreferences;
use crate::controls::MAX_CUSTOM_HZ;

impl DisplayPreferences {
    pub fn is_valid_custom_hz(hz: u32) -> bool {
        (1..=MAX_CUSTOM_HZ).contains(&hz)
    }

    /// Adds a custom rate, kept sorted and without duplicates. `false` when nothing changed.
    pub fn add_custom_hz(&mut self, connector: &str, hz: u32) -> bool {
        if !Self::is_valid_custom_hz(hz) {
            return false;
        }
        let rates = self.custom_hz.entry(connector.to_owned()).or_default();
        if rates.contains(&hz) {
            return false;
        }
        rates.push(hz);
        rates.sort_unstable();
        true
    }

    /// Removes a custom rate and forgets the connector once it has none left.
    pub fn remove_custom_hz(&mut self, connector: &str, hz: u32) -> bool {
        let Some(rates) = self.custom_hz.get_mut(connector) else {
            return false;
        };
        let before = rates.len();
        rates.retain(|&r| r != hz);
        let changed = rates.len() != before;
        if rates.is_empty() {
            self.custom_hz.remove(connector);
        }
        changed
    }

    pub fn custom_hz_of(&self, connector: &str) -> &[u32] {
        self.custom_hz.get(connector).map_or(&[], Vec::as_slice)
    }
}
