use std::fmt;

use crate::value::refresh::interfaces::RefreshCentiHz;

/// Kanshi token fragment: integer when whole, otherwise two decimals.
impl fmt::Display for RefreshCentiHz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_multiple_of(100) {
            write!(f, "{}", self.0 / 100)
        } else {
            f.write_str(&self.fixed2())
        }
    }
}
