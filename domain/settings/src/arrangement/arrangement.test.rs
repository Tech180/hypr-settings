use crate::arrangement::interfaces::{Layout, Rect, Tile};
use crate::value::{ConnectorName, Position};

fn name(s: &str) -> ConnectorName {
    ConnectorName::try_from(s).unwrap()
}

fn layout(tiles: &[(&str, i32, i32, i32, i32)]) -> Layout {
    Layout::new(
        tiles
            .iter()
            .map(|(n, x, y, w, h)| Tile {
                connector: name(n),
                rect: Rect::new(*x, *y, *w, *h),
            })
            .collect(),
    )
}

fn two() -> Layout {
    layout(&[("eDP-1", 0, 0, 1920, 1080), ("DP-3", 1920, 0, 2560, 1080)])
}

#[test]
fn touching_edges_do_not_overlap_but_are_adjacent() {
    let l = two();
    assert!(l.overlaps().is_empty());
    assert!(l.is_connected());
    let a = Rect::new(0, 0, 100, 100);
    assert!(!a.overlaps(Rect::new(100, 0, 100, 100)));
    assert!(a.adjacent(Rect::new(100, 50, 100, 100)));
    assert!(!a.adjacent(Rect::new(100, 100, 100, 100)), "corner only");
    assert!(a.overlaps(Rect::new(99, 99, 10, 10)));
}

#[test]
fn gap_makes_a_layout_disconnected() {
    let l = layout(&[("A", 0, 0, 100, 100), ("B", 110, 0, 100, 100)]);
    assert!(!l.is_connected());
    assert!(l.overlaps().is_empty());
}

#[test]
fn drop_next_to_a_neighbour_snaps_flush() {
    let l = two();
    let p = l.resolve_drop(&name("DP-3"), (2560, 1080), Position::new(1950, 20), 30);
    assert_eq!(p, Position::new(1920, 0), "flush right and top aligned");
}

#[test]
fn single_neighbour_always_sticks() {
    let l = two();
    let far = Position::new(9000, 5000);
    let p = l.resolve_drop(&name("DP-3"), (2560, 1080), far, 30);
    assert!(l.accepts(&name("DP-3"), (2560, 1080), p));
    assert_eq!(
        p.y, 1080,
        "far below and right leans downward, so it sits below"
    );
    assert_eq!(
        p.x,
        1920 - 64,
        "and slides along that edge as far as it can"
    );
}

#[test]
fn dragging_across_a_wide_neighbour_switches_sides_once() {
    let l = two();
    let mut sides = Vec::new();
    for x in (-1000..6000).step_by(50) {
        let p = l.resolve_drop(&name("eDP-1"), (1920, 1080), Position::new(x, 0), 30);
        let side = match p.x {
            0 => "left",
            4480 => "right",
            other => panic!("unexpected x {other} for candidate {x}"),
        };
        if sides.last() != Some(&side) {
            sides.push(side);
        }
    }
    assert_eq!(sides, ["left", "right"]);
}

#[test]
fn alignment_snaps_to_monitors_that_are_not_touched() {
    let l = layout(&[
        ("A", 0, 0, 1000, 1000),
        ("B", 1000, 600, 1000, 1000),
        ("C", 5000, 5000, 500, 500),
    ]);
    let p = l.resolve_drop(&name("C"), (500, 500), Position::new(2000, 510), 30);
    assert_eq!(
        p,
        Position::new(2000, 500),
        "bottom aligned with A across the layout"
    );
}

#[test]
fn moving_the_bridge_monitor_pulls_the_stranded_one_back() {
    let mut l = layout(&[
        ("A", 0, 0, 1000, 1000),
        ("B", 0, 1000, 1000, 1000),
        ("C", 2000, 0, 1000, 1000),
    ]);
    assert!(!l.is_connected());
    l.reconnect(&name("B"));
    assert!(l.is_connected());
    assert_eq!(l.get(&name("A")), Some(Rect::new(0, 0, 1000, 1000)));
    assert_eq!(l.get(&name("B")), Some(Rect::new(0, 1000, 1000, 1000)));
    assert_eq!(l.get(&name("C")), Some(Rect::new(1000, 0, 1000, 1000)));
    assert!(l.overlaps().is_empty());
}

#[test]
fn reconnect_leaves_a_connected_layout_alone() {
    let mut l = two();
    l.reconnect(&name("DP-3"));
    assert_eq!(l, two());
}

#[test]
fn drop_into_an_overlap_slides_to_a_legal_edge() {
    let l = two();
    let p = l.resolve_drop(&name("DP-3"), (2560, 1080), Position::new(500, 100), 0);
    let rect = Rect::at(p, (2560, 1080));
    assert!(!rect.overlaps(Rect::new(0, 0, 1920, 1080)));
    assert!(l.accepts(&name("DP-3"), (2560, 1080), p));
}

#[test]
fn drop_below_snaps_alignment_within_threshold() {
    let l = two();
    let p = l.resolve_drop(&name("eDP-1"), (1920, 1080), Position::new(1950, 1100), 30);
    assert_eq!(p, Position::new(1920, 1080), "below DP-3, left aligned");
}

#[test]
fn validated_drop_keeps_legal_positions_and_fixes_illegal_ones() {
    let l = two();
    let legal = Position::new(1920, 300);
    assert_eq!(
        l.validated_drop(&name("DP-3"), (2560, 1080), legal),
        legal,
        "partial vertical overlap with the neighbour is legal"
    );
    let overlapping = Position::new(1000, 0);
    let fixed = l.validated_drop(&name("DP-3"), (2560, 1080), overlapping);
    assert_ne!(fixed, overlapping);
    assert!(l.accepts(&name("DP-3"), (2560, 1080), fixed));
    let gap = Position::new(2100, 0);
    assert!(!l.accepts(&name("DP-3"), (2560, 1080), gap));
    assert_eq!(
        l.validated_drop(&name("DP-3"), (2560, 1080), gap),
        Position::new(1920, 0)
    );
}

#[test]
fn resizing_pushes_right_and_bottom_neighbours() {
    // eDP-1 rotates from 1920x1080 to 1080x1920: DP-3 follows its right edge.
    let mut l = two();
    l.reflow_resize(&name("eDP-1"), (1080, 1920));
    assert_eq!(l.get(&name("eDP-1")), Some(Rect::new(0, 0, 1080, 1920)));
    assert_eq!(l.get(&name("DP-3")), Some(Rect::new(1080, 0, 2560, 1080)));
    assert!(l.overlaps().is_empty() && l.is_connected());
}

#[test]
fn growing_pushes_a_chain_and_vertical_neighbours() {
    let mut l = layout(&[
        ("A", 0, 0, 1000, 1000),
        ("B", 1000, 0, 1000, 1000),
        ("C", 2000, 0, 1000, 1000),
        ("D", 0, 1000, 1000, 500),
    ]);
    l.reflow_resize(&name("A"), (1500, 1200));
    assert_eq!(l.get(&name("B")).unwrap().x, 1500);
    assert_eq!(l.get(&name("C")).unwrap().x, 2500, "transitive follower");
    assert_eq!(l.get(&name("D")).unwrap().y, 1200);
    assert!(l.overlaps().is_empty() && l.is_connected());
}

#[test]
fn shrinking_pulls_followers_in_and_keeps_the_layout_connected() {
    let mut l = two();
    l.reflow_resize(&name("eDP-1"), (1280, 720));
    assert_eq!(l.get(&name("DP-3")).unwrap().x, 1280);
    assert!(l.is_connected());
}

#[test]
fn left_neighbour_stays_when_the_right_one_changes() {
    let mut l = two();
    l.reflow_resize(&name("DP-3"), (1920, 1080));
    assert_eq!(l.get(&name("eDP-1")), Some(Rect::new(0, 0, 1920, 1080)));
    assert_eq!(l.get(&name("DP-3")), Some(Rect::new(1920, 0, 1920, 1080)));
}

#[test]
fn stray_overlaps_are_pushed_apart() {
    // C sits diagonally and is only hit once A grows in both directions.
    let mut l = layout(&[("A", 0, 0, 100, 100), ("C", 100, 100, 100, 100)]);
    l.reflow_resize(&name("A"), (150, 150));
    assert!(l.overlaps().is_empty());
}

#[test]
fn place_new_prefers_a_legal_saved_position() {
    let l = layout(&[("A", 0, 0, 1920, 1080)]);
    let saved = Position::new(1920, 100);
    assert_eq!(l.place_new((1280, 720), Some(saved)), saved);
    assert_eq!(
        l.place_new((1280, 720), Some(Position::new(500, 0))),
        Position::new(1920, 0),
        "overlapping saved position falls back to the right edge"
    );
    assert_eq!(
        l.place_new((1280, 720), Some(Position::new(5000, 0))),
        Position::new(1920, 0),
        "a detached saved position is not reused"
    );
    assert_eq!(
        Layout::default().place_new((1, 1), None),
        Position::new(0, 0)
    );
}

#[test]
fn normalise_moves_the_union_to_the_origin() {
    let mut l = layout(&[("A", -1080, -300, 1080, 1920), ("B", 0, 0, 1920, 1080)]);
    assert_eq!(l.normalise_origin(), (1080, 300));
    assert_eq!(l.get(&name("A")).unwrap().position(), Position::new(0, 0));
    assert_eq!(
        l.get(&name("B")).unwrap().position(),
        Position::new(1080, 300)
    );
}

#[test]
fn mock_layout_is_legal() {
    // The sample layout from previews/display.html, in logical pixels.
    let l = layout(&[
        ("DP-3", -1080, -300, 1080, 1920),
        ("DP-6", 0, 0, 1920, 1080),
        ("eDP-1", 0, 1080, 1920, 1200),
    ]);
    assert!(l.overlaps().is_empty());
    assert!(l.is_connected());
}
