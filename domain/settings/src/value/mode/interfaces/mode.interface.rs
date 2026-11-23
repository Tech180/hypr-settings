use serde::{Deserialize, Serialize};

use crate::value::{RefreshCentiHz, Resolution};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Mode {
    pub resolution: Resolution,
    pub refresh: RefreshCentiHz,
}
