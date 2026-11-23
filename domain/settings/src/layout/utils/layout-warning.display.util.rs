use std::fmt;

use crate::layout::interfaces::LayoutWarning;

impl fmt::Display for LayoutWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => {
                f.write_str("Some displays do not touch, so the pointer cannot cross between them")
            }
        }
    }
}
