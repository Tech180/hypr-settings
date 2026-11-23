//! Pure state machine for a trial: apply a change, then keep it or revert to the
//! full previous `ProfileSet`. Time enters as `Instant` values supplied by the caller.

use std::time::{Duration, Instant};

use crate::error::TrialError;
use crate::layout::{LayoutPatch, ModeRequest};
use crate::profile::ProfileSet;
use crate::value::ConnectorName;

use crate::trial::interfaces::{Trial, TrialKind, TrialPolicy, TrialSlot};

impl TrialKind {
    /// `None` when the patch only toggles enable (applied without a trial).
    pub fn from_patch(patch: &LayoutPatch) -> Option<Self> {
        match patch.mode {
            Some(ModeRequest::Refresh(_)) => Some(Self::Refresh),
            Some(_) => Some(Self::Resolution),
            None if patch.scale.is_some() => Some(Self::Scale),
            None if patch.transform.is_some() => Some(Self::Transform),
            None if patch.position.is_some() => Some(Self::Position),
            None => None,
        }
    }
}

impl Default for TrialPolicy {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
        }
    }
}

impl Trial {
    pub fn remaining(&self, now: Instant) -> Duration {
        self.deadline.saturating_duration_since(now)
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        now >= self.deadline
    }
}

impl TrialSlot {
    pub fn current(&self) -> Option<&Trial> {
        self.0.as_ref()
    }

    pub fn begin(
        &mut self,
        now: Instant,
        policy: TrialPolicy,
        connector: ConnectorName,
        kind: TrialKind,
        before: ProfileSet,
    ) -> Result<&Trial, TrialError> {
        if self.0.is_some() {
            return Err(TrialError::InProgress);
        }
        Ok(self.0.insert(Trial {
            connector,
            kind,
            started: now,
            deadline: now + policy.timeout,
            before,
        }))
    }

    /// Accepts the change; the applied conf is already persisted.
    pub fn keep(&mut self) -> Option<Trial> {
        self.0.take()
    }

    /// Ends the trial and hands back the profiles to restore.
    pub fn revert(&mut self) -> Option<ProfileSet> {
        self.0.take().map(|t| t.before)
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        self.0.as_ref().is_some_and(|t| t.is_expired(now))
    }
}
