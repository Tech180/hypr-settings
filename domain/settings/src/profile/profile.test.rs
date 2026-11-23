use crate::layout::{LayoutPatch, LayoutPolicy, ModeRequest, ResolvedPatch};
use crate::output::fixtures::*;
use crate::output::{OutputIdentity, Topology};
use crate::profile::interfaces::*;
use crate::profile::{ProfileSet, glob_match};
use crate::value::{ConnectorName, Position, Scale, Transform};

fn name(s: &str) -> ProfileName {
    ProfileName::parse(s).unwrap()
}

fn conn(s: &str) -> ConnectorName {
    ConnectorName::try_from(s).unwrap()
}

fn profile(n: &str, outputs: &[&str], rest_glob: bool) -> Profile {
    let mut p = Profile::new(name(n));
    for o in outputs {
        p.outputs.push(ProfileOutput {
            criteria: Criteria::parse(o),
            enable: Some(true),
            mode: Some("1920x1080@60".parse().unwrap()),
            position: Some(Position::new(0, 0)),
            scale: Some(Scale::clamped(1.0)),
            transform: Some(Transform::Normal),
            rest_glob: false,
            extra: Vec::new(),
        });
    }
    if rest_glob {
        let mut glob = p.outputs[0].clone();
        glob.criteria = Criteria::Any;
        glob.rest_glob = true;
        p.outputs.push(glob);
    }
    p
}

fn seed() -> ProfileSet {
    ProfileSet::new(vec![
        profile("home", &["eDP-1", "DP-3", "DP-6"], false),
        profile("portable", &["eDP-1"], false),
        profile("asia", &["eDP-1", "DP-3"], false),
        profile("docked", &["eDP-1"], true),
        profile("default", &["*"], false),
    ])
}

fn id(connector: &str, make: &str, model: &str, serial: &str) -> OutputIdentity {
    OutputIdentity {
        connector: conn(connector),
        make: make.into(),
        model: model.into(),
        serial: serial.into(),
    }
}

fn connected(names: &[&str]) -> Vec<OutputIdentity> {
    names.iter().map(|n| id(n, "", "", "")).collect()
}

#[test]
fn first_matching_profile_in_file_order_wins() {
    let set = seed();
    assert_eq!(
        set.match_active(&connected(&["eDP-1", "DP-3"])),
        Some(name("asia"))
    );
    assert_eq!(
        set.match_active(&connected(&["eDP-1"])),
        Some(name("portable"))
    );

    // Same heads, catch-all first: kanshi would pick the catch-all.
    let mut reordered = seed();
    let docked = reordered.profiles.remove(3);
    reordered.profiles.insert(0, docked);
    assert_eq!(
        reordered.match_active(&connected(&["eDP-1", "DP-3"])),
        Some(name("docked"))
    );
}

#[test]
fn catch_all_needs_an_extra_output() {
    let set = seed();
    assert_eq!(
        set.match_active(&connected(&["eDP-1", "HDMI-A-1"])),
        Some(name("docked"))
    );
    assert_eq!(set.match_active(&connected(&["DP-9"])), None);
}

#[test]
fn rest_glob_absorbs_any_number_of_extras() {
    let set = seed();
    assert_eq!(
        set.match_active(&connected(&["eDP-1", "HDMI-A-1", "DP-7", "DP-8"])),
        Some(name("docked"))
    );
}

#[test]
fn single_star_matches_exactly_one_output() {
    let mut set = ProfileSet::new(vec![profile("pair", &["eDP-1", "*"], false)]);
    assert_eq!(
        set.match_active(&connected(&["eDP-1", "DP-3"])),
        Some(name("pair"))
    );
    assert_eq!(set.match_active(&connected(&["eDP-1"])), None);
    assert_eq!(
        set.match_active(&connected(&["eDP-1", "DP-3", "DP-4"])),
        None
    );
    set.profiles[0].outputs[1].criteria = Criteria::Named("DP-3".into());
    assert_eq!(set.match_active(&connected(&["eDP-1", "DP-4"])), None);
}

#[test]
fn default_is_never_matched() {
    let set = ProfileSet::new(vec![profile("default", &["eDP-1"], false)]);
    assert_eq!(set.match_active(&connected(&["eDP-1"])), None);
}

#[test]
fn description_criteria_match_make_model_serial() {
    // The seed `home` profile names its Dell monitors by description.
    let set = ProfileSet::new(vec![profile(
        "home",
        &[
            "eDP-1",
            "Dell Inc. DELL P2422H B95JF04",
            "Dell Inc. DELL P2422H 9BWHF04",
        ],
        false,
    )]);
    let heads = [
        id("eDP-1", "BOE", "0x0A89", ""),
        id("DP-9", "Dell Inc.", "DELL P2422H", "9BWHF04"),
        id("DP-2", "Dell Inc.", "DELL P2422H", "B95JF04"),
    ];
    assert_eq!(set.match_active(&heads), Some(name("home")));
    let other = [
        id("eDP-1", "BOE", "0x0A89", ""),
        id("DP-9", "Dell Inc.", "DELL P2422H", "ZZZZZZZ"),
        id("DP-2", "Dell Inc.", "DELL P2422H", "B95JF04"),
    ];
    assert_eq!(set.match_active(&other), None);
}

#[test]
fn globs_and_unknown_serials_follow_kanshi() {
    let heads = [id("DP-1", "Foocorp", "ASDF", "")];
    let matches = |c: &str| Criteria::parse(c).matches(&heads[0]);
    assert!(matches("Foocorp ASDF Unknown"));
    assert!(matches("Foocorp ASDF "), "blank serial spelled empty");
    assert!(matches("Foocorp ASDF *"));
    assert!(matches("Foo* ?SDF *"));
    assert!(matches("DP-1"));
    assert!(matches("*"));
    assert!(!matches("Foocorp ASDF 1234"));
    assert!(!matches("DP-2"));
    assert!(!matches("$work-desk"), "aliases are kanshi's business");
    assert!(!matches("Foocorp"), "descriptions need all three fields");
}

#[test]
fn glob_basics() {
    assert!(glob_match("a*c", "abbbc"));
    assert!(glob_match("a?c", "abc"));
    assert!(!glob_match("a?c", "ac"));
    assert!(glob_match("*", ""));
    assert!(!glob_match("abc", "abcd"));
}

fn resolved(topo: &Topology, patch: LayoutPatch) -> ResolvedPatch {
    LayoutPolicy::validate(topo, &patch).unwrap()
}

#[test]
fn upsert_updates_only_listed_outputs() {
    let topo = Topology::new(vec![laptop(), ultrawide()]);
    let patch = resolved(
        &topo,
        LayoutPatch::new(conn("DP-3")).mode(ModeRequest::Refresh(100)),
    );
    let mut set = seed();
    assert!(set.upsert_layout(&name("asia"), &topo, Some(&patch)).stored);
    let asia = set.get(&name("asia")).unwrap();
    assert_eq!(asia.outputs.len(), 2);
    assert_eq!(
        asia.outputs[1].mode.unwrap().kanshi_token(),
        "2560x1080@100"
    );
    assert_eq!(asia.outputs[1].position, Some(Position::new(1920, 0)));
}

#[test]
fn upsert_writes_the_reflowed_position_of_every_output() {
    let topo = Topology::new(vec![laptop(), ultrawide()]);
    let patch = resolved(
        &topo,
        LayoutPatch::new(conn("eDP-1")).transform(Transform::R90),
    );
    let mut set = seed();
    set.upsert_layout(&name("asia"), &topo, Some(&patch));
    let asia = set.get(&name("asia")).unwrap();
    assert_eq!(asia.outputs[0].transform, Some(Transform::R90));
    assert_eq!(asia.outputs[0].position, Some(Position::new(0, 0)));
    assert_eq!(asia.outputs[1].position, Some(Position::new(1080, 0)));
}

#[test]
fn upsert_matches_slots_by_description_not_only_connector() {
    let mut dp = ultrawide();
    dp.make = "Dell Inc.".into();
    dp.model = "DELL P2422H".into();
    dp.serial = "B95JF04".into();
    dp.connector = conn("DP-9");
    let topo = Topology::new(vec![laptop(), dp]);
    let patch = resolved(
        &topo,
        LayoutPatch::new(conn("DP-9")).position(Position::new(1920, 300)),
    );
    let mut set = ProfileSet::new(vec![profile(
        "home",
        &["eDP-1", "Dell Inc. DELL P2422H B95JF04"],
        false,
    )]);
    assert!(set.upsert_layout(&name("home"), &topo, Some(&patch)).stored);
    let out = &set.get(&name("home")).unwrap().outputs[1];
    assert_eq!(
        out.criteria,
        Criteria::Named("Dell Inc. DELL P2422H B95JF04".into())
    );
    assert_eq!(out.position, Some(Position::new(1920, 300)));
}

#[test]
fn edit_under_a_catch_all_is_promoted_to_a_specific_profile_before_it() {
    let mut other = ultrawide();
    other.connector = conn("HDMI-A-1");
    other.make = "Acme".into();
    other.model = "Wide".into();
    other.serial = "42".into();
    let topo = Topology::new(vec![laptop(), other]);
    let patch = resolved(
        &topo,
        LayoutPatch::new(conn("HDMI-A-1")).position(Position::new(1920, 200)),
    );
    let mut set = seed();
    let outcome = set.upsert_layout(&name("docked"), &topo, Some(&patch));
    assert!(outcome.stored);
    assert_eq!(outcome.profile, name("docked-2"));

    let docked_at = set.profiles.iter().position(|p| p.name == name("docked"));
    let promoted_at = set.profiles.iter().position(|p| p.name == outcome.profile);
    assert!(promoted_at < docked_at, "specific profile must come first");

    let docked = set.get(&name("docked")).unwrap();
    assert_eq!(docked.outputs.len(), 2, "catch-all is untouched");
    assert!(docked.outputs[1].rest_glob);

    let promoted = set.get(&outcome.profile).unwrap();
    assert_eq!(promoted.outputs.len(), 2);
    assert_eq!(
        promoted.outputs[0].criteria,
        Criteria::Named("eDP-1".into())
    );
    assert_eq!(
        promoted.outputs[1].criteria,
        Criteria::Named("Acme Wide 42".into())
    );
    assert_eq!(promoted.outputs[1].position, Some(Position::new(1920, 200)));

    // And kanshi, reading the file top to bottom, now selects it.
    assert_eq!(set.match_active(&topo.identities()), Some(outcome.profile));
}

#[test]
fn promotion_names_do_not_collide() {
    let mut other = ultrawide();
    other.connector = conn("HDMI-A-1");
    let topo = Topology::new(vec![laptop(), other]);
    let patch = resolved(&topo, LayoutPatch::new(conn("HDMI-A-1")).enable(true));
    let mut set = seed();
    set.profiles
        .insert(0, profile("docked-2", &["DP-1"], false));
    let outcome = set.upsert_layout(&name("docked"), &topo, Some(&patch));
    assert_eq!(outcome.profile, name("docked-3"));
}

#[test]
fn plain_edit_of_a_named_output_leaves_a_catch_all_alone() {
    let mut other = ultrawide();
    other.connector = conn("HDMI-A-1");
    let topo = Topology::new(vec![laptop(), other]);
    let patch = resolved(
        &topo,
        LayoutPatch::new(conn("eDP-1")).mode(ModeRequest::Refresh(48)),
    );
    let mut set = seed();
    let outcome = set.upsert_layout(&name("docked"), &topo, Some(&patch));
    assert_eq!(outcome.profile, name("docked"));
    assert_eq!(set.profiles.len(), 5);
    assert!(set.get(&name("docked")).unwrap().outputs[1].rest_glob);
}

#[test]
fn explicit_normal_transform_is_written() {
    let topo = Topology::new(vec![laptop()]);
    let patch = resolved(
        &topo,
        LayoutPatch::new(conn("eDP-1")).transform(Transform::Normal),
    );
    let mut set = seed();
    set.upsert_layout(&name("portable"), &topo, Some(&patch));
    let out = &set.get(&name("portable")).unwrap().outputs[0];
    assert_eq!(out.transform, Some(Transform::Normal));
}

#[test]
fn re_enable_keeps_prior_mode_when_live_mode_is_gone() {
    let mut gone = ultrawide();
    gone.enabled = false;
    gone.mode = None;
    let topo = Topology::new(vec![laptop(), gone]);
    let patch = resolved(&topo, LayoutPatch::new(conn("DP-3")).enable(true));
    let mut set = seed();
    set.profiles[2].outputs[1].enable = Some(false);
    assert!(set.upsert_layout(&name("asia"), &topo, Some(&patch)).stored);
    let out = &set.get(&name("asia")).unwrap().outputs[1];
    assert_eq!(out.enable, Some(true));
    assert!(out.mode.is_some());
    assert_eq!(out.position, Some(Position::new(1920, 0)));
}

#[test]
fn disabling_keeps_the_saved_position() {
    let topo = Topology::new(vec![laptop(), ultrawide()]);
    let patch = resolved(&topo, LayoutPatch::new(conn("DP-3")).enable(false));
    let mut set = seed();
    set.profiles[2].outputs[1].position = Some(Position::new(1920, 77));
    set.upsert_layout(&name("asia"), &topo, Some(&patch));
    let out = &set.get(&name("asia")).unwrap().outputs[1];
    assert_eq!(out.enable, Some(false));
    assert_eq!(out.position, Some(Position::new(1920, 77)));
}

#[test]
fn home_snapshot_replaces_with_live_topology() {
    let topo = Topology::new(vec![laptop(), ultrawide()]);
    let mut set = ProfileSet::new(vec![profile("default", &["*"], false)]);
    assert!(set.upsert_home_full_snapshot(&topo, None));
    assert_eq!(set.profiles[0].name, name("home"));
    assert_eq!(set.profiles[0].outputs.len(), 2);
    assert_eq!(set.profiles[1].name, name("default"));
}

#[test]
fn snapshots_name_displays_by_description_when_it_is_stable() {
    let mut dp = ultrawide();
    dp.make = "Sceptre Tech Inc".into();
    dp.model = "Sceptre P30".into();
    dp.serial = "0000000000000".into();
    let mut twin_a = ultrawide();
    twin_a.connector = conn("DP-6");
    twin_a.make = "Twin".into();
    twin_a.model = "T".into();
    twin_a.serial = "1".into();
    let mut twin_b = twin_a.clone();
    twin_b.connector = conn("DP-7");
    let topo = Topology::new(vec![laptop(), dp, twin_a, twin_b]);
    let mut set = ProfileSet::new(vec![profile("default", &["*"], false)]);
    assert!(set.snapshot_to_profile(&name("desk"), &topo));
    let desk = set.get(&name("desk")).unwrap();
    let criteria: Vec<_> = desk.outputs.iter().map(|o| o.criteria.as_str()).collect();
    assert_eq!(
        criteria,
        [
            "eDP-1",
            "Sceptre Tech Inc Sceptre P30 0000000000000",
            "DP-6",
            "DP-7"
        ],
        "built-in by connector, unique displays by description, twins by connector"
    );
}

#[test]
fn snapshot_of_catch_all_refreshes_named_entries_only() {
    let topo = Topology::new(vec![laptop(), ultrawide()]);
    let mut set = seed();
    assert!(set.snapshot_to_profile(&name("docked"), &topo));
    let docked = set.get(&name("docked")).unwrap();
    assert_eq!(docked.outputs.len(), 2);
    assert!(docked.outputs[1].rest_glob);
}

#[test]
fn extras_survive_a_layout_rewrite_except_replaced_keys() {
    let topo = Topology::new(vec![laptop()]);
    let mut set = seed();
    let out = &mut set.profiles[1].outputs[0];
    out.extra = vec![
        "adaptive_sync".into(),
        "on".into(),
        "mode".into(),
        "--custom".into(),
        "1x1@1".into(),
    ];
    set.upsert_layout(&name("portable"), &topo, None);
    assert_eq!(
        set.profiles[1].outputs[0].extra,
        vec!["adaptive_sync", "on"]
    );
}
