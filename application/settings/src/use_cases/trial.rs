use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use settings_domain::{
    LayoutPatch, Topology, Trial, TrialError, TrialKind, TrialPolicy, TrialRecord, TrialSlot,
};

use super::apply_layout::{apply_layout, plan, restore_profiles};
use crate::dto::{ApplyOutcome, RecoverOutcome, TickOutcome, TrialView};
use crate::error::AppError;
use crate::ports::Ports;

/// The single running trial plus what is needed to undo and verify it.
pub(crate) struct TrialState {
    pub slot: TrialSlot,
    pub outcome: ApplyOutcome,
    /// The topology as it was before the trial, for verifying a revert.
    pub original: Option<Topology>,
    /// A layout change is being written and verified; the slot is filled only after.
    pub applying: bool,
}

impl Default for TrialState {
    fn default() -> Self {
        Self {
            slot: TrialSlot::default(),
            outcome: ApplyOutcome::Verified,
            original: None,
            applying: false,
        }
    }
}

/// Marks a change as in flight for as long as it lives, so callers arriving meanwhile
/// are turned away at once instead of queueing behind it.
pub(crate) struct Applying<'a>(&'a Mutex<TrialState>);

impl<'a> Applying<'a> {
    pub(crate) fn start(state: &'a Mutex<TrialState>) -> Result<Self, AppError> {
        let mut st = lock(state);
        if st.applying || st.slot.current().is_some() {
            return Err(TrialError::InProgress.into());
        }
        st.applying = true;
        Ok(Self(state))
    }
}

impl Drop for Applying<'_> {
    fn drop(&mut self) {
        lock(self.0).applying = false;
    }
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn trial_view(trial: &Trial, outcome: ApplyOutcome, now: Instant) -> TrialView {
    TrialView {
        connector: trial.connector.to_string(),
        kind: trial.kind,
        remaining_secs: trial.remaining(now).as_millis().div_ceil(1000) as u64,
        timeout_secs: trial
            .deadline
            .saturating_duration_since(trial.started)
            .as_secs(),
        outcome,
    }
}

pub(crate) fn current_view(ports: &Ports, state: &Mutex<TrialState>) -> Option<TrialView> {
    let now = ports.clock.now();
    let st = lock(state);
    st.slot.current().map(|t| trial_view(t, st.outcome, now))
}

pub(crate) fn ensure_idle(state: &Mutex<TrialState>) -> Result<(), AppError> {
    let st = lock(state);
    if st.applying || st.slot.current().is_some() {
        return Err(TrialError::InProgress.into());
    }
    Ok(())
}

/// Write-ahead journal first, then apply; a failure undoes everything it touched.
/// The caller holds an [`Applying`] guard for the duration.
pub(crate) fn begin(
    ports: &Ports,
    policy: TrialPolicy,
    state: &Mutex<TrialState>,
    patch: LayoutPatch,
) -> Result<TrialView, AppError> {
    let (topology, resolved) = plan(ports, &patch)?;
    let kind = TrialKind::from_patch(&patch).ok_or_else(|| {
        AppError::Rejected("no mode, scale, rotation or position change was requested".into())
    })?;

    let before = ports.profiles.load()?;
    let original = Some(topology.clone());
    ports.journal.write(&TrialRecord {
        connector: patch.connector.clone(),
        before: before.clone(),
        deadline_unix: ports.clock.unix_secs() + policy.timeout.as_secs(),
    })?;

    let outcome = match apply_layout(ports, &topology, &resolved) {
        Ok(outcome) => outcome,
        Err(err) => {
            if let Err(restore_err) = restore_profiles(ports, &before, original.as_ref()) {
                tracing::error!(%restore_err, "could not restore profiles after a failed trial");
            } else if let Err(clear_err) = ports.journal.clear() {
                tracing::warn!(%clear_err, "could not clear trial journal");
            }
            return Err(err);
        }
    };

    let now = ports.clock.now();
    let mut st = lock(state);
    let trial = st.slot.begin(now, policy, patch.connector, kind, before)?;
    let view = trial_view(trial, outcome, now);
    st.outcome = outcome;
    st.original = original;
    Ok(view)
}

pub(crate) fn keep(ports: &Ports, state: &Mutex<TrialState>) -> Result<(), AppError> {
    if lock(state).slot.keep().is_none() {
        return Err(AppError::Rejected("no trial is running".into()));
    }
    ports.journal.clear()?;
    Ok(())
}

/// The journal stays when restoring fails, so the next start retries.
pub(crate) fn revert(ports: &Ports, state: &Mutex<TrialState>) -> Result<(), AppError> {
    let (before, original) = {
        let mut st = lock(state);
        let before = st
            .slot
            .revert()
            .ok_or_else(|| AppError::Rejected("no trial is running".into()))?;
        (before, st.original.take())
    };
    restore_profiles(ports, &before, original.as_ref())?;
    ports.journal.clear()?;
    Ok(())
}

pub(crate) fn tick(ports: &Ports, state: &Mutex<TrialState>) -> Result<TickOutcome, AppError> {
    let now = ports.clock.now();
    let snapshot = {
        let st = lock(state);
        st.slot
            .current()
            .map(|t| (t.is_expired(now), trial_view(t, st.outcome, now)))
    };
    match snapshot {
        None => Ok(TickOutcome::Idle),
        Some((false, view)) => Ok(TickOutcome::Active(view)),
        Some((true, _)) => {
            revert(ports, state)?;
            Ok(TickOutcome::Expired)
        }
    }
}

/// Undo a trial left behind by a crashed process.
pub(crate) fn recover(
    ports: &Ports,
    state: &Mutex<TrialState>,
) -> Result<RecoverOutcome, AppError> {
    if lock(state).slot.current().is_some() {
        return Ok(RecoverOutcome::Nothing);
    }
    let Some(record) = ports.journal.read()? else {
        return Ok(RecoverOutcome::Nothing);
    };
    ports.profiles.save(&record.before)?;
    ports.controller.reload()?;
    ports.journal.clear()?;
    Ok(RecoverOutcome::Restored {
        connector: record.connector.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_in_flight_turns_others_away_until_it_ends() {
        let state = Mutex::new(TrialState::default());
        let first = Applying::start(&state).unwrap();
        assert!(matches!(
            Applying::start(&state),
            Err(AppError::Trial(TrialError::InProgress))
        ));
        assert!(ensure_idle(&state).is_err());
        drop(first);
        assert!(ensure_idle(&state).is_ok());
        assert!(Applying::start(&state).is_ok());
    }
}
