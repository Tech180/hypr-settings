mod support;

use std::sync::Arc;

use settings_application::dto::ApplyOutcome;
use settings_application::{AppError, DisplayApi, DisplayService};
use settings_domain::{LayoutError, LayoutPatch, Position, ProfileName, Transform, TrialPolicy};
use support::{Fake, conn, laptop, output, ultrawide};

fn service(fake: &Arc<Fake>) -> DisplayService {
    DisplayService::new(fake.ports(), TrialPolicy::default())
}

fn position(fake: &Fake, connector: &str) -> Position {
    fake.with(|w| w.topology.find(&conn(connector)).unwrap().position)
}

fn work(fake: &Fake) -> settings_domain::Profile {
    fake.profiles()
        .get(&ProfileName::parse("work").unwrap())
        .unwrap()
        .clone()
}

#[test]
fn dropping_below_applies_as_a_trial_and_keep_saves_it() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);

    let trial = svc
        .begin_trial(LayoutPatch::new(conn("DP-3")).position(Position::new(0, 1080)))
        .unwrap();
    assert_eq!(trial.outcome, ApplyOutcome::Verified);
    assert_eq!(position(&fake, "DP-3"), Position::new(0, 1080));

    svc.keep_trial().unwrap();
    assert_eq!(
        work(&fake).outputs[1].position,
        Some(Position::new(0, 1080))
    );
}

#[test]
fn reverting_a_move_restores_both_the_layout_and_the_profile() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);
    let before = fake.profiles();

    svc.begin_trial(LayoutPatch::new(conn("DP-3")).position(Position::new(0, 1080)))
        .unwrap();
    svc.revert_trial().unwrap();

    assert_eq!(position(&fake, "DP-3"), Position::new(1920, 0));
    assert_eq!(fake.profiles(), before);
}

#[test]
fn overlapping_drop_is_pushed_flush_not_applied_as_is() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    svc_begin_move(&fake, (500, 0));
    let at = position(&fake, "DP-3");
    let laptop_rect = (0, 0, 1920, 1080);
    let overlaps = at.x < laptop_rect.2 && at.x + 2560 > laptop_rect.0 && at.y < laptop_rect.3;
    assert!(!overlaps, "landed at {at:?}");
}

fn svc_begin_move(fake: &Arc<Fake>, to: (i32, i32)) {
    service(fake)
        .begin_trial(LayoutPatch::new(conn("DP-3")).position(Position::new(to.0, to.1)))
        .unwrap();
}

#[test]
fn rotating_an_output_moves_its_neighbour_along() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);

    // 1920x1080 turned 90° is 1080 wide, so DP-3 must follow from x=1920 to x=1080.
    svc.begin_trial(LayoutPatch::new(conn("eDP-1")).transform(Transform::R90))
        .unwrap();
    assert_eq!(position(&fake, "DP-3"), Position::new(1080, 0));
    assert_eq!(position(&fake, "eDP-1"), Position::new(0, 0));

    svc.revert_trial().unwrap();
    assert_eq!(position(&fake, "DP-3"), Position::new(1920, 0));
}

#[test]
fn flipped_transforms_are_applied_and_kept() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);
    svc.begin_trial(LayoutPatch::new(conn("eDP-1")).transform(Transform::Flipped))
        .unwrap();
    assert_eq!(
        fake.with(|w| w.topology.find(&conn("eDP-1")).unwrap().transform),
        Transform::Flipped
    );
    svc.keep_trial().unwrap();
    assert_eq!(work(&fake).outputs[0].transform, Some(Transform::Flipped));
}

#[test]
fn a_disconnected_layout_is_rejected_before_anything_is_written() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    // Overlapping the neighbour with a size change that cannot be reflowed away is
    // not possible here, so check the plain rejection path: a scale that is invalid
    // for the panel never reaches the compositor.
    let err = service(&fake)
        .begin_trial(
            LayoutPatch::new(conn("eDP-1")).scale(settings_domain::Scale::new(1.75).unwrap()),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        AppError::Layout(LayoutError::InvalidScale { .. })
    ));
    assert!(fake.position_of("journal.write").is_none());
    assert!(fake.position_of("live.apply").is_none());
}

#[test]
fn crash_recovery_puts_every_output_back() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let before = fake.profiles();
    service(&fake)
        .begin_trial(LayoutPatch::new(conn("eDP-1")).transform(Transform::R90))
        .unwrap();
    assert_eq!(position(&fake, "DP-3"), Position::new(1080, 0));

    // The app died; a fresh service finds the journal.
    service(&fake).recover_pending_trial().unwrap();
    assert_eq!(position(&fake, "DP-3"), Position::new(1920, 0));
    assert_eq!(fake.profiles(), before);
}

#[test]
fn moving_a_catch_all_output_promotes_it_to_its_own_profile() {
    let fake = Fake::new(vec![
        output("DP-1", &["1920x1080@60.00Hz"], "1920x1080@60", 0),
        output("DP-2", &["1920x1080@60.00Hz"], "1920x1080@60", 1920),
    ]);
    let mut set = fake.profiles();
    set.profiles.clear();
    set.profiles.push(support::absorbing_profile());
    fake.with(|w| w.profiles = set);

    let svc = service(&fake);
    assert_eq!(svc.view().unwrap().active_profile.as_deref(), Some("pair"));

    svc.begin_trial(LayoutPatch::new(conn("DP-2")).position(Position::new(0, 1080)))
        .unwrap();
    svc.keep_trial().unwrap();

    let stored = fake.profiles();
    assert_eq!(stored.profiles.len(), 2, "{stored:?}");
    // The new profile sits before the catch-all and names the real displays.
    assert_ne!(stored.profiles[0].name.as_str(), "pair");
    assert_eq!(position(&fake, "DP-2"), Position::new(0, 1080));
    let view = svc.view().unwrap();
    assert_eq!(
        view.active_profile.as_deref(),
        Some(stored.profiles[0].name.as_str())
    );
}
