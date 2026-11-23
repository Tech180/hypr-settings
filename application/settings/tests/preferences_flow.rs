mod support;

use std::sync::Arc;

use settings_application::{AppError, DisplayApi, DisplayService};
use settings_domain::{LayoutError, ProfileName, TrialPolicy};
use support::{Fake, conn, laptop, ultrawide};

fn service(fake: &Arc<Fake>) -> DisplayService {
    DisplayService::new(fake.ports(), TrialPolicy::default())
}

#[test]
fn the_bar_defaults_to_the_laptop_and_follows_the_choice() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);
    assert_eq!(svc.view().unwrap().bar_host.as_deref(), Some("eDP-1"));

    svc.set_bar_host(conn("DP-3")).unwrap();
    assert_eq!(svc.view().unwrap().bar_host.as_deref(), Some("DP-3"));
    assert_eq!(
        fake.with(|w| w.prefs.bar_host_connector.clone()).as_deref(),
        Some("DP-3")
    );
}

#[test]
fn a_display_that_is_off_cannot_host_the_bar() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);
    svc.set_enabled(conn("DP-3"), false).unwrap();
    assert!(matches!(
        svc.set_bar_host(conn("DP-3")),
        Err(AppError::Rejected(_))
    ));
}

#[test]
fn the_bar_host_cannot_be_switched_off() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);
    svc.set_bar_host(conn("DP-3")).unwrap();
    let err = svc.set_enabled(conn("DP-3"), false).unwrap_err();
    assert!(
        matches!(err, AppError::Layout(LayoutError::BarHost(_))),
        "{err:?}"
    );

    svc.set_enabled(conn("eDP-1"), false).unwrap();
}

#[test]
fn custom_rates_show_up_as_removable_chips_when_advertised() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);
    svc.add_custom_hz(conn("DP-3"), 144).unwrap();
    svc.add_custom_hz(conn("DP-3"), 75).unwrap();
    svc.add_custom_hz(conn("DP-3"), 100).unwrap();

    let view = svc.view().unwrap();
    let chips = &view
        .outputs
        .iter()
        .find(|o| o.connector == "DP-3")
        .unwrap()
        .hz_options;
    let custom: Vec<u32> = chips.iter().filter(|c| c.custom).map(|c| c.hz).collect();
    assert_eq!(
        custom,
        [75, 100],
        "144 Hz is not advertised, so it stays hidden"
    );

    svc.remove_custom_hz(conn("DP-3"), 75).unwrap();
    assert!(fake.with(|w| w.prefs.custom_hz["DP-3"] == [100, 144]));
    assert!(matches!(
        svc.add_custom_hz(conn("DP-3"), 0),
        Err(AppError::Rejected(_))
    ));
}

#[test]
fn saving_goes_to_the_active_profile_or_home() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    let svc = service(&fake);
    assert_eq!(svc.save_to_active_profile().unwrap(), "work");

    let lone = Fake::new(vec![support::output(
        "HDMI-A-1",
        &["1920x1080@60.00Hz"],
        "1920x1080@60",
        0,
    )]);
    let svc = service(&lone);
    let name = svc.save_to_active_profile().unwrap();
    assert_ne!(name, "work");
    assert!(
        lone.profiles()
            .get(&ProfileName::parse(&name).unwrap())
            .is_some()
    );
}
