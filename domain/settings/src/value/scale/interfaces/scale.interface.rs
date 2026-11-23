use serde::{Deserialize, Serialize};

/// Output scale between 0.5 and 4.0, stored in hundredths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Scale(pub(crate) u16);
