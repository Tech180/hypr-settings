mod support;

use std::time::Duration;

use settings_application::dto::{ApplyOutcome, RecoverOutcome, TickOutcome};
use settings_application::{AppError, DisplayApi, DisplayService};
use settings_domain::{
    LayoutError, LayoutPatch, ModeRequest, ProfileName, Scale, Transform, TrialError, TrialPolicy,
    TrialRecord,
};
use support::{Fake, conn, laptop, ultrawide};

fn service(fake: &std::sync::Arc<Fake>) -> DisplayService {
    DisplayService::new(fake.ports(), TrialPolicy::default())
}

fn refresh_100() -> LayoutPatch {
    LayoutPatch::new(conn("DP-3")).mode(ModeRequest::Refresh(100))
}

#[test]
fn trial_then_keep_persists_the_change() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);

    let trial = svc.begin_trial(refresh_100()).unwrap();
    assert_eq!(trial.outcome, ApplyOutcome::Verified);
    assert_eq!(trial.timeout_secs, 30);
    assert_eq!(fake.current_mode("DP-3").refresh.rounded_hz(), 100);
    assert!(fake.with(|w| w.journal.is_some()));

    svc.keep_trial().unwrap();
    assert!(fake.with(|w| w.journal.is_none()));
    let stored = fake.profiles();
    let work = stored.get(&ProfileName::parse("work").unwrap()).unwrap();
    assert_eq!(
        work.outputs[1].mode.unwrap().kanshi_token(),
        "2560x1080@100"
    );
    assert!(svc.view().unwrap().trial.is_none());
}

#[test]
fn expiry_restores_the_previous_layout() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);
    let before = fake.profiles();

    svc.begin_trial(refresh_100()).unwrap();
    assert!(matches!(svc.tick().unwrap(), TickOutcome::Active(_)));

    fake.advance(Duration::from_secs(31));
    assert_eq!(svc.tick().unwrap(), TickOutcome::Expired);
    assert_eq!(fake.current_mode("DP-3").refresh.rounded_hz(), 60);
    assert_eq!(fake.profiles(), before);
    assert!(fake.with(|w| w.journal.is_none()));
    assert_eq!(svc.tick().unwrap(), TickOutcome::Idle);
}

#[test]
fn explicit_revert_restores_too() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);
    let before = fake.profiles();

    svc.begin_trial(LayoutPatch::new(conn("eDP-1")).scale(Scale::new(1.5).unwrap()))
        .unwrap();
    assert_eq!(
        fake.with(|w| w.topology.find(&conn("eDP-1")).unwrap().scale.hundredths()),
        150
    );

    svc.revert_trial().unwrap();
    assert_eq!(
        fake.with(|w| w.topology.find(&conn("eDP-1")).unwrap().scale.hundredths()),
        100
    );
    assert_eq!(fake.profiles(), before);
}

#[test]
fn failed_reload_falls_back_to_the_compositor() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    fake.with(|w| w.reload_fails = true);
    let svc = service(&fake);

    let trial = svc.begin_trial(refresh_100()).unwrap();
    assert_eq!(trial.outcome, ApplyOutcome::Degraded);
    assert_eq!(fake.current_mode("DP-3").refresh.rounded_hz(), 100);

    svc.revert_trial().unwrap();
    assert_eq!(fake.current_mode("DP-3").refresh.rounded_hz(), 60);
    assert!(
        fake.log().iter().filter(|l| *l == "live.apply").count() >= 2,
        "degraded apply and revert both touch the compositor"
    );
}

#[test]
fn unconfirmed_apply_is_reported() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    fake.with(|w| w.reload_changes_nothing = true);
    let trial = service(&fake).begin_trial(refresh_100()).unwrap();
    assert_eq!(trial.outcome, ApplyOutcome::Unverified);
}

#[test]
fn second_trial_is_rejected() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);
    svc.begin_trial(refresh_100()).unwrap();
    assert_eq!(
        svc.begin_trial(LayoutPatch::new(conn("eDP-1")).scale(Scale::new(2.0).unwrap())),
        Err(AppError::Trial(TrialError::InProgress))
    );
    assert!(matches!(
        svc.set_enabled(conn("DP-3"), false),
        Err(AppError::Trial(TrialError::InProgress))
    ));
}

#[test]
fn leftover_journal_is_undone_on_recovery() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let original = fake.profiles();
    let mut crashed = original.clone();
    crashed.profiles[0].outputs[1].mode = Some(support::mode("2560x1080@100"));
    fake.with(|w| {
        w.profiles = crashed;
        w.journal = Some(TrialRecord {
            connector: conn("DP-3"),
            before: original.clone(),
            deadline_unix: 1,
        });
    });

    let outcome = service(&fake).recover_pending_trial().unwrap();
    assert_eq!(
        outcome,
        RecoverOutcome::Restored {
            connector: "DP-3".into()
        }
    );
    assert_eq!(fake.profiles(), original);
    assert!(fake.with(|w| w.journal.is_none()));
    assert_eq!(
        service(&fake).recover_pending_trial().unwrap(),
        RecoverOutcome::Nothing
    );
}

#[test]
fn journal_is_written_before_profiles_change() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    service(&fake).begin_trial(refresh_100()).unwrap();
    let journal = fake.position_of("journal.write").unwrap();
    let save = fake.position_of("store.save").unwrap();
    assert!(journal < save);
}

#[test]
fn invalid_request_touches_nothing() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let err = service(&fake)
        .begin_trial(LayoutPatch::new(conn("DP-3")).mode(ModeRequest::Refresh(144)))
        .unwrap_err();
    assert!(matches!(
        err,
        AppError::Layout(LayoutError::UnsupportedMode { .. })
    ));
    assert!(fake.position_of("journal.write").is_none());
    assert!(fake.position_of("store.save").is_none());
}

#[test]
fn last_display_cannot_be_disabled() {
    let fake = Fake::new(vec![laptop()]);
    let err = service(&fake)
        .set_enabled(conn("eDP-1"), false)
        .unwrap_err();
    assert_eq!(
        err,
        AppError::Layout(LayoutError::LastDisplay(conn("eDP-1")))
    );
}

#[test]
fn enable_toggle_goes_through_the_profile() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let outcome = service(&fake).set_enabled(conn("DP-3"), false).unwrap();
    assert_eq!(outcome, ApplyOutcome::Verified);
    assert!(!fake.with(|w| w.topology.find(&conn("DP-3")).unwrap().enabled));
}

#[test]
fn view_describes_outputs_and_profiles() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let view = service(&fake).view().unwrap();

    assert_eq!(view.active_profile.as_deref(), Some("work"));
    assert_eq!(view.profiles.iter().filter(|p| p.active).count(), 1);

    let dp3 = view.outputs.iter().find(|o| o.connector == "DP-3").unwrap();
    assert!(dp3.ultrawide);
    assert_eq!(
        dp3.hz_options.iter().map(|c| c.hz).collect::<Vec<_>>(),
        vec![60, 75, 100]
    );
    assert_eq!(dp3.hz_options.iter().filter(|c| c.active).count(), 1);
    assert_eq!(dp3.resolutions[0].width, 2560);
    assert!(dp3.resolutions[0].active);
    assert_eq!(dp3.scale_options.len(), 5);
    assert_eq!(dp3.transform_options.len(), Transform::ALL.len());
}
