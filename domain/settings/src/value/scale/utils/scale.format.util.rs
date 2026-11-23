use std::fmt;

use crate::value::scale::interfaces::Scale;

/// Renders like kanshi expects: `1`, `1.5`, `1.25` (no trailing `×`; use [`Scale::label`](Scale::label) in the UI).
impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hundredths = self.0;
        if hundredths.is_multiple_of(100) {
            write!(f, "{}", hundredths / 100)
        } else if hundredths.is_multiple_of(10) {
            write!(f, "{}.{}", hundredths / 100, (hundredths % 100) / 10)
        } else {
            write!(f, "{}.{:02}", hundredths / 100, hundredths % 100)
        }
    }
}
