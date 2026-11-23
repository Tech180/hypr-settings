mod support;

use std::sync::Arc;

use settings_application::{AppError, ControlsApi, DisplayApi, DisplayService};
use settings_domain::controls::{
    BrightnessTarget, DrmProbe, Flag, InputSources, VCP_BRIGHTNESS, VCP_CONTRAST,
};
use settings_domain::{ProfileName, TrialPolicy};
use support::{Fake, conn, ddc_display, input, laptop, ultrawide};

fn service(fake: &Arc<Fake>) -> DisplayService {
    DisplayService::new(fake.hardware_ports(), TrialPolicy::default())
}

/// A laptop with a backlight and an external display reachable over DDC/CI on bus 5.
fn desk() -> Arc<Fake> {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    fake.with(|w| {
        w.backlight = Some(40);
        w.ddc_displays = vec![ddc_display(5, "DP-3")];
        w.ddc_values.insert((5, VCP_BRIGHTNESS), 70);
        w.ddc_values.insert((5, VCP_CONTRAST), 30);
    });
    fake
}

#[test]
fn a_synchronised_change_reaches_every_display_and_is_shared() {
    let fake = desk();
    let svc = service(&fake);
    svc.set_brightness(80).unwrap();

    assert_eq!(fake.with(|w| w.backlight), Some(80));
    assert_eq!(fake.with(|w| w.ddc_values[&(5, VCP_BRIGHTNESS)]), 80);
    let shared = fake.with(|w| w.brightness_state.clone()).unwrap();
    assert_eq!(shared.percent, 80);
    assert_eq!(
        shared.targets,
        [BrightnessTarget::Internal, BrightnessTarget::Ddc { bus: 5 }]
    );
    assert_eq!(shared.host, Some(BrightnessTarget::Internal));
}

#[test]
fn without_sync_only_the_bar_host_changes() {
    let fake = desk();
    let svc = service(&fake);
    svc.set_sync_brightness(false).unwrap();
    svc.set_bar_host(conn("DP-3")).unwrap();
    svc.set_brightness(10).unwrap();

    assert_eq!(fake.with(|w| w.ddc_values[&(5, VCP_BRIGHTNESS)]), 10);
    assert_eq!(fake.with(|w| w.backlight), Some(40));
    assert!(!svc.brightness().unwrap().sync);
}

#[test]
fn adjusting_follows_the_sync_preference_and_clamps() {
    let fake = desk();
    let svc = service(&fake);
    svc.set_sync_brightness(false).unwrap();
    // The host is the laptop (40 %); the other display is left alone.
    assert_eq!(svc.adjust_brightness(5).unwrap(), 45);
    assert_eq!(fake.with(|w| w.backlight), Some(45));
    assert_eq!(fake.with(|w| w.ddc_values[&(5, VCP_BRIGHTNESS)]), 70);

    svc.set_sync_brightness(true).unwrap();
    assert_eq!(svc.adjust_brightness(-100).unwrap(), 0);
    assert_eq!(fake.with(|w| w.ddc_values[&(5, VCP_BRIGHTNESS)]), 0);
    assert_eq!(svc.adjust_brightness(500).unwrap(), 100);
}

#[test]
fn the_keys_work_from_the_shared_targets_without_looking_at_the_displays() {
    let fake = desk();
    let svc = service(&fake);
    svc.set_brightness(50).unwrap();
    // A display disappears from the machine, but the keys do not look again.
    fake.with(|w| w.ddc_displays.clear());
    svc.adjust_brightness(10).unwrap();
    assert_eq!(fake.with(|w| w.ddc_values[&(5, VCP_BRIGHTNESS)]), 60);

    let view = svc.refresh_brightness().unwrap();
    assert_eq!(view.targets, 1, "refreshing notices it");
}

#[test]
fn moving_the_bar_updates_the_host_the_keys_use() {
    let fake = desk();
    let svc = service(&fake);
    svc.set_sync_brightness(false).unwrap();
    svc.brightness().unwrap();
    svc.set_bar_host(conn("DP-3")).unwrap();
    assert_eq!(
        fake.with(|w| w.brightness_state.clone().unwrap().host),
        Some(BrightnessTarget::Ddc { bus: 5 })
    );
}

#[test]
fn brightness_fails_cleanly_when_nothing_can_change() {
    let fake = Fake::new(vec![ultrawide()]);
    let svc = service(&fake);
    assert!(matches!(svc.set_brightness(50), Err(AppError::Rejected(_))));
    assert_eq!(svc.brightness().unwrap().percent, None);
}

#[test]
fn output_controls_report_values_and_why_others_are_locked() {
    let fake = desk();
    fake.with(|w| {
        w.overdrive_code = Some(0xf0);
        w.ddc_values.insert((5, 0xf0), 2);
        w.inputs = InputSources {
            current: Some(input("DP 1", 0x0f)),
            options: vec![input("DP 1", 0x0f), input("HDMI 1", 0x11)],
        };
    });
    let svc = service(&fake);

    let external = svc.output_controls(conn("DP-3")).unwrap();
    assert_eq!(external.brightness.percent, Some(70));
    assert_eq!(external.contrast.percent, Some(30));
    assert_eq!(external.overdrive.percent, Some(2));
    assert_eq!(external.input.options.len(), 2);
    assert_eq!(external.port.unwrap().sink_input.as_deref(), Some("DP 1"));

    let panel = svc.output_controls(conn("eDP-1")).unwrap();
    assert_eq!(panel.brightness.percent, Some(40));
    assert!(panel.contrast.locked);
    assert!(panel.overdrive.locked);
    assert!(panel.input.options.is_empty());
}

#[test]
fn contrast_overdrive_and_input_go_to_the_matching_bus() {
    let fake = desk();
    fake.with(|w| {
        w.overdrive_code = Some(0xf0);
        w.inputs.options = vec![input("HDMI 1", 0x11)];
    });
    let svc = service(&fake);
    svc.set_contrast(conn("DP-3"), 55).unwrap();
    svc.set_overdrive(conn("DP-3"), 3).unwrap();
    svc.set_input_source(conn("DP-3"), 0x11).unwrap();

    assert_eq!(fake.with(|w| w.ddc_values[&(5, VCP_CONTRAST)]), 55);
    assert_eq!(fake.with(|w| w.ddc_values[&(5, 0xf0)]), 3);
    assert!(fake.log().contains(&"ddc.input 5 0x11".to_owned()));
    assert!(matches!(
        svc.set_contrast(conn("eDP-1"), 10),
        Err(AppError::Rejected(_))
    ));
}

#[test]
fn night_light_follows_the_override_and_survives_a_restart() {
    let fake = Fake::new(vec![laptop()]);
    let svc = service(&fake);
    svc.set_night_kelvin(500).unwrap();
    assert_eq!(svc.night_light().unwrap().kelvin, 3000);
    assert!(fake.with(|w| w.night).is_none(), "off: only remembered");

    svc.set_night_light(true).unwrap();
    assert_eq!(fake.with(|w| w.night.map(|k| k.get())), Some(3000));
    svc.set_night_kelvin(4500).unwrap();
    assert_eq!(fake.with(|w| w.night.map(|k| k.get())), Some(4500));

    fake.with(|w| w.night = None);
    svc.restore_night_light().unwrap();
    assert_eq!(fake.with(|w| w.night.map(|k| k.get())), Some(4500));

    svc.set_night_light(false).unwrap();
    assert!(fake.with(|w| w.night).is_none());
    svc.restore_night_light().unwrap();
    assert!(fake.with(|w| w.night).is_none());
}

fn capable() -> Arc<Fake> {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    fake.with(|w| {
        w.drm.insert(
            "DP-3".into(),
            DrmProbe {
                vrr_capable: Some(true),
                max_bpc: Some(10),
                ..DrmProbe::default()
            },
        );
    });
    fake
}

#[test]
fn a_flag_is_applied_verified_and_stored_for_the_profile() {
    let fake = capable();
    let svc = service(&fake);
    svc.set_flag(conn("DP-3"), Flag::Vrr, true).unwrap();

    assert!(fake.with(|w| w.topology.find(&conn("DP-3")).unwrap().vrr));
    let stored = fake.with(|w| w.flags.clone());
    assert_eq!(stored["work"]["DP-3"].vrr, Some(true));
}

#[test]
fn unsupported_flags_are_refused_but_a_live_one_can_be_turned_off() {
    let fake = capable();
    let svc = service(&fake);
    let err = svc.set_flag(conn("DP-3"), Flag::Hdr, true).unwrap_err();
    assert!(matches!(err, AppError::Rejected(m) if m.contains("No HDR metadata")));

    fake.with(|w| {
        let mut outputs = w.topology.outputs().to_vec();
        outputs[1].hdr = true;
        w.topology = settings_domain::Topology::new(outputs);
    });
    svc.set_flag(conn("DP-3"), Flag::Hdr, false).unwrap();
    assert!(!fake.with(|w| w.topology.find(&conn("DP-3")).unwrap().hdr));
}

#[test]
fn a_flag_the_compositor_ignores_is_put_back() {
    let fake = capable();
    fake.with(|w| w.reject_flags = true);
    let svc = service(&fake);
    let err = svc.set_flag(conn("DP-3"), Flag::Vrr, true).unwrap_err();
    assert!(matches!(err, AppError::Rejected(m) if m.contains("put back")));
    assert!(!fake.with(|w| w.topology.find(&conn("DP-3")).unwrap().vrr));
    assert!(fake.with(|w| w.flags.is_empty()), "nothing is stored");
}

#[test]
fn switching_profiles_restores_the_stored_flags() {
    let fake = capable();
    let svc = service(&fake);
    svc.set_flag(conn("DP-3"), Flag::Vrr, true).unwrap();
    fake.with(|w| {
        let mut outputs = w.topology.outputs().to_vec();
        outputs[1].vrr = false;
        w.topology = settings_domain::Topology::new(outputs);
    });

    svc.switch_profile(ProfileName::parse("work").unwrap())
        .unwrap();
    assert!(fake.with(|w| w.topology.find(&conn("DP-3")).unwrap().vrr));
}

#[test]
fn restore_skips_what_the_display_cannot_do() {
    let fake = Fake::new(vec![laptop(), ultrawide()]);
    fake.with(|w| {
        w.flags.entry("work".into()).or_default().insert(
            "DP-3".into(),
            settings_domain::controls::OutputFlags {
                hdr: Some(true),
                ..Default::default()
            },
        );
    });
    let svc = service(&fake);
    assert_eq!(svc.restore_flags().unwrap(), 0);
    assert!(!fake.with(|w| w.topology.find(&conn("DP-3")).unwrap().hdr));
}

#[test]
fn saving_a_profile_snapshots_the_live_flags() {
    let fake = capable();
    let svc = service(&fake);
    svc.set_flag(conn("DP-3"), Flag::Vrr, true).unwrap();
    svc.save_snapshot_as(ProfileName::parse("gaming").unwrap())
        .unwrap();
    let stored = fake.with(|w| w.flags.clone());
    assert_eq!(stored["gaming"]["DP-3"].vrr, Some(true));
    assert_eq!(stored["gaming"]["eDP-1"].vrr, Some(false));

    svc.delete_profile(ProfileName::parse("gaming").unwrap())
        .unwrap();
    assert!(!fake.with(|w| w.flags.contains_key("gaming")));
}
