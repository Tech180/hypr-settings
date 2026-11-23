use serde::{Deserialize, Serialize};

/// Refresh rate in hundredths of a hertz. Advertised modes carry two decimals,
/// so integers keep equality exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RefreshCentiHz(pub(crate) u32);
