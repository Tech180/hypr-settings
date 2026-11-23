use std::time::{Duration, Instant};

use crate::error::TrialError;
use crate::layout::{LayoutPatch, ModeRequest};
use crate::profile::ProfileSet;
use crate::trial::TrialKind;
use crate::trial::interfaces::{TrialPolicy, TrialSlot};
use crate::value::ConnectorName;

#[allow(clippy::disallowed_methods)]
fn base() -> Instant {
    Instant::now()
}

fn conn() -> ConnectorName {
    ConnectorName::try_from("eDP-1").unwrap()
}

fn begin(slot: &mut TrialSlot, now: Instant) -> Result<(), TrialError> {
    slot.begin(
        now,
        TrialPolicy::default(),
        conn(),
        TrialKind::Refresh,
        ProfileSet::default(),
    )
    .map(|_| ())
}

#[test]
fn second_trial_is_rejected() {
    let mut slot = TrialSlot::default();
    let now = base();
    begin(&mut slot, now).unwrap();
    assert_eq!(begin(&mut slot, now), Err(TrialError::InProgress));
}

#[test]
fn expires_after_the_timeout() {
    let mut slot = TrialSlot::default();
    let now = base();
    begin(&mut slot, now).unwrap();
    assert!(!slot.is_expired(now + Duration::from_secs(29)));
    assert!(slot.is_expired(now + Duration::from_secs(30)));
    assert_eq!(
        slot.current()
            .unwrap()
            .remaining(now + Duration::from_secs(10)),
        Duration::from_secs(20)
    );
}

#[test]
fn keep_and_revert_free_the_slot() {
    let mut slot = TrialSlot::default();
    let now = base();
    begin(&mut slot, now).unwrap();
    assert!(slot.keep().is_some());
    begin(&mut slot, now).unwrap();
    assert_eq!(slot.revert(), Some(ProfileSet::default()));
    assert!(slot.current().is_none());
}

#[test]
fn enable_only_patch_has_no_trial_kind() {
    let patch = LayoutPatch::new(conn()).enable(false);
    assert_eq!(TrialKind::from_patch(&patch), None);
    let patch = LayoutPatch::new(conn()).mode(ModeRequest::Refresh(60));
    assert_eq!(TrialKind::from_patch(&patch), Some(TrialKind::Refresh));
}
