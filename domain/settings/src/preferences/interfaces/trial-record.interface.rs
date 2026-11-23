use serde::{Deserialize, Serialize};

use crate::profile::ProfileSet;
use crate::value::ConnectorName;

/// Write-ahead record of a running trial, used to undo it after a crash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrialRecord {
    pub connector: ConnectorName,
    pub before: ProfileSet,
    pub deadline_unix: u64,
}
