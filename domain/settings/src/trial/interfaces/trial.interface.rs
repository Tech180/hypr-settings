use std::time::Instant;

use crate::profile::ProfileSet;
use crate::value::ConnectorName;

use super::trial_kind::TrialKind;

#[derive(Debug, Clone, PartialEq)]
pub struct Trial {
    pub connector: ConnectorName,
    pub kind: TrialKind,
    pub started: Instant,
    pub deadline: Instant,
    /// Everything needed to undo the trial: the profiles as they were before it began.
    pub before: ProfileSet,
}
