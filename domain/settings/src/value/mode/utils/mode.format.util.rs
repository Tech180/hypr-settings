use std::fmt;

use crate::value::mode::interfaces::Mode;

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.resolution, self.refresh)
    }
}
