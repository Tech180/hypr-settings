use crate::error::LayoutError;
use crate::layout::interfaces::*;
use crate::output::Topology;
use crate::output::fixtures::*;
use crate::value::{ConnectorName, Mode, Position, Scale, Transform};

fn topo() -> Topology {
    Topology::new(vec![laptop(), ultrawide()])
}

fn name(s: &str) -> ConnectorName {
    ConnectorName::try_from(s).unwrap()
}

#[test]
fn moving_to_the_left_is_stored_from_the_origin() {
    let patch = LayoutPatch::new(name("DP-3")).position(Position::new(-2560, 0));
    let resolved = LayoutPolicy::validate(&topo(), &patch).unwrap();
    assert_eq!(pos(&resolved, "DP-3"), Position::new(0, 0));
    assert_eq!(pos(&resolved, "eDP-1"), Position::new(2560, 0));
    assert_eq!(resolved.position, Some(Position::new(0, 0)));
}

#[test]
fn geometry_free_edits_keep_an_offset_layout_where_it_is() {
    let mut edp = laptop();
    edp.position = Position::new(100, 50);
    let mut dp = ultrawide();
    dp.position = Position::new(2020, 50);
    let topology = Topology::new(vec![edp, dp]);
    let patch = LayoutPatch::new(name("DP-3")).mode(ModeRequest::Refresh(100));
    let resolved = LayoutPolicy::validate(&topology, &patch).unwrap();
    assert_eq!(pos(&resolved, "eDP-1"), Position::new(100, 50));
    assert_eq!(pos(&resolved, "DP-3"), Position::new(2020, 50));
}

#[test]
fn disabling_the_bridge_display_pulls_the_far_one_back() {
    let mut right = ultrawide();
    right.connector = name("DP-4");
    right.position = Position::new(4480, 0);
    let topology = Topology::new(vec![laptop(), ultrawide(), right]);
    let patch = LayoutPatch::new(name("DP-3")).enable(false);
    let resolved = LayoutPolicy::validate(&topology, &patch).unwrap();
    assert_eq!(pos(&resolved, "eDP-1"), Position::new(0, 0));
    assert_eq!(pos(&resolved, "DP-4"), Position::new(1920, 0));
    assert!(resolved.warnings.is_empty());
}

#[test]
fn refresh_request_resolves_to_advertised_fraction() {
    let patch = LayoutPatch::new(name("DP-3")).mode(ModeRequest::Refresh(100));
    let resolved = LayoutPolicy::validate(&topo(), &patch).unwrap();
    assert_eq!(resolved.mode.unwrap().kanshi_token(), "2560x1080@100");
}

#[test]
fn unadvertised_refresh_is_rejected() {
    let patch = LayoutPatch::new(name("DP-3")).mode(ModeRequest::Refresh(144));
    assert!(matches!(
        LayoutPolicy::validate(&topo(), &patch),
        Err(LayoutError::UnsupportedMode { .. })
    ));
}

#[test]
fn resolution_change_keeps_rate_or_falls_back() {
    let keep = LayoutPatch::new(name("DP-3")).mode(ModeRequest::Resolution(res(1920, 1080)));
    let resolved = LayoutPolicy::validate(&topo(), &keep).unwrap();
    assert_eq!(resolved.mode.unwrap().kanshi_token(), "1920x1080@60");

    let missing = LayoutPatch::new(name("DP-3")).mode(ModeRequest::Resolution(res(800, 600)));
    assert!(LayoutPolicy::validate(&topo(), &missing).is_err());
}

#[test]
fn last_enabled_display_cannot_be_disabled() {
    let single = Topology::new(vec![laptop()]);
    let patch = LayoutPatch::new(name("eDP-1")).enable(false);
    assert_eq!(
        LayoutPolicy::validate(&single, &patch),
        Err(LayoutError::LastDisplay(name("eDP-1")))
    );
    assert!(LayoutPolicy::validate(&topo(), &patch).is_ok());
}

fn pos(resolved: &ResolvedPatch, connector: &str) -> Position {
    resolved.position_of(&name(connector)).unwrap()
}

#[test]
fn untouched_layout_keeps_every_position() {
    let patch = LayoutPatch::new(name("DP-3")).mode(ModeRequest::Refresh(100));
    let resolved = LayoutPolicy::validate(&topo(), &patch).unwrap();
    assert_eq!(pos(&resolved, "eDP-1"), Position::new(0, 0));
    assert_eq!(pos(&resolved, "DP-3"), Position::new(1920, 0));
    assert!(resolved.warnings.is_empty());
}

#[test]
fn rotating_the_left_monitor_shifts_its_neighbour() {
    let patch = LayoutPatch::new(name("eDP-1")).transform(Transform::R90);
    let resolved = LayoutPolicy::validate(&topo(), &patch).unwrap();
    assert_eq!(pos(&resolved, "eDP-1"), Position::new(0, 0));
    assert_eq!(pos(&resolved, "DP-3"), Position::new(1080, 0));
}

#[test]
fn mirrored_quarter_turns_swap_axes_too() {
    let patch = LayoutPatch::new(name("eDP-1")).transform(Transform::Flipped270);
    let resolved = LayoutPolicy::validate(&topo(), &patch).unwrap();
    assert_eq!(pos(&resolved, "DP-3"), Position::new(1080, 0));
    let flat = LayoutPatch::new(name("eDP-1")).transform(Transform::Flipped);
    let resolved = LayoutPolicy::validate(&topo(), &flat).unwrap();
    assert_eq!(pos(&resolved, "DP-3"), Position::new(1920, 0));
}

#[test]
fn scale_changes_logical_size_and_reflow() {
    let patch = LayoutPatch::new(name("eDP-1")).scale(Scale::new(2.0).unwrap());
    let resolved = LayoutPolicy::validate(&topo(), &patch).unwrap();
    assert_eq!(pos(&resolved, "DP-3"), Position::new(960, 0));
}

#[test]
fn scale_that_leaves_fractional_pixels_is_refused_with_a_suggestion() {
    let patch = LayoutPatch::new(name("eDP-1")).scale(Scale::new(1.75).unwrap());
    assert_eq!(
        LayoutPolicy::validate(&topo(), &patch),
        Err(LayoutError::InvalidScale {
            connector: name("eDP-1"),
            requested: Scale::new(1.75).unwrap(),
            nearest: Scale::new(1.6).unwrap(),
        })
    );
}

#[test]
fn mode_change_repairs_a_scale_the_new_mode_cannot_hold() {
    let mut laptop = laptop();
    laptop.scale = Scale::new(1.25).unwrap();
    laptop.modes = ["1920x1080@60.00Hz", "1366x768@60.00Hz"]
        .iter()
        .map(|m| m.parse::<Mode>().unwrap())
        .collect();
    let topology = Topology::new(vec![laptop]);
    let patch = LayoutPatch::new(name("eDP-1")).mode(ModeRequest::Resolution(res(1366, 768)));
    let resolved = LayoutPolicy::validate(&topology, &patch).unwrap();
    let fixed = resolved.scale.unwrap();
    assert!(fixed.is_valid_for(res(1366, 768)));
}

#[test]
fn dragging_to_an_overlap_is_snapped_to_a_legal_edge() {
    let patch = LayoutPatch::new(name("DP-3")).position(Position::new(500, 0));
    let resolved = LayoutPolicy::validate(&topo(), &patch).unwrap();
    let at = pos(&resolved, "DP-3");
    assert!(at.x >= 1920 || at.x + 2560 <= 0 || at.y >= 1080 || at.y + 1080 <= 0);
}

#[test]
fn dragging_to_a_legal_position_is_kept() {
    let patch = LayoutPatch::new(name("DP-3")).position(Position::new(1920, 540));
    let resolved = LayoutPolicy::validate(&topo(), &patch).unwrap();
    assert_eq!(pos(&resolved, "DP-3"), Position::new(1920, 540));
    assert_eq!(resolved.position, Some(Position::new(1920, 540)));
}

#[test]
fn disabling_keeps_the_others_where_they_are() {
    let patch = LayoutPatch::new(name("eDP-1")).enable(false);
    let resolved = LayoutPolicy::validate(&topo(), &patch).unwrap();
    assert_eq!(resolved.position_of(&name("eDP-1")), None);
    assert_eq!(pos(&resolved, "DP-3"), Position::new(1920, 0));
}

#[test]
fn re_enabling_reuses_the_saved_position_when_it_is_still_free() {
    let mut off = laptop();
    off.enabled = false;
    off.mode = None;
    let topology = Topology::new(vec![off, {
        let mut dp = ultrawide();
        dp.position = Position::new(1920, 0);
        dp
    }]);
    let patch = LayoutPatch::new(name("eDP-1")).enable(true);
    let resolved = LayoutPolicy::validate(&topology, &patch).unwrap();
    assert_eq!(pos(&resolved, "eDP-1"), Position::new(0, 0));
    assert_eq!(pos(&resolved, "DP-3"), Position::new(1920, 0));
}

#[test]
fn re_enabling_over_a_taken_spot_goes_beside_the_layout() {
    let mut off = laptop();
    off.enabled = false;
    off.mode = None;
    off.position = Position::new(1000, 0);
    let mut dp = ultrawide();
    dp.position = Position::new(0, 0);
    let topology = Topology::new(vec![off, dp]);
    let patch = LayoutPatch::new(name("eDP-1")).enable(true);
    let resolved = LayoutPolicy::validate(&topology, &patch).unwrap();
    assert_eq!(pos(&resolved, "eDP-1"), Position::new(2560, 0));
}

#[test]
fn existing_overlaps_do_not_block_unrelated_changes() {
    let mut dp = ultrawide();
    dp.position = Position::new(1000, 0);
    let topology = Topology::new(vec![laptop(), dp]);
    let patch = LayoutPatch::new(name("DP-3")).mode(ModeRequest::Refresh(100));
    // Both tiles keep their rects, so the stale overlap is not this patch's fault.
    assert!(LayoutPolicy::validate(&topology, &patch).is_ok());
}

#[test]
fn detached_layouts_are_allowed_with_a_warning() {
    let mut dp = ultrawide();
    dp.position = Position::new(5000, 0);
    let topology = Topology::new(vec![laptop(), dp]);
    let patch = LayoutPatch::new(name("eDP-1")).mode(ModeRequest::Refresh(48));
    let resolved = LayoutPolicy::validate(&topology, &patch).unwrap();
    assert_eq!(resolved.warnings, vec![LayoutWarning::Disconnected]);
}

#[test]
fn unknown_output_is_rejected() {
    let patch = LayoutPatch::new(name("DP-9")).enable(true);
    assert!(matches!(
        LayoutPolicy::validate(&topo(), &patch),
        Err(LayoutError::UnknownOutput(_))
    ));
}

#[test]
fn the_bar_host_cannot_be_switched_off() {
    let patch = LayoutPatch::new(name("DP-3")).enable(false);
    let err = LayoutPolicy::guard_bar_host(&topo(), &patch, Some("DP-3")).unwrap_err();
    assert!(matches!(err, LayoutError::BarHost(_)));
    assert!(LayoutPolicy::guard_bar_host(&topo(), &patch, Some("eDP-1")).is_ok());
    assert!(LayoutPolicy::guard_bar_host(&topo(), &patch, None).is_ok());
    let move_only = LayoutPatch::new(name("DP-3")).position(Position::new(0, 0));
    assert!(LayoutPolicy::guard_bar_host(&topo(), &move_only, Some("DP-3")).is_ok());
}
