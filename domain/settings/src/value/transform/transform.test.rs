use crate::value::transform::Transform;

#[test]
fn hypr_round_trip_keeps_flips() {
    assert_eq!(Transform::from_hypr(5), Transform::Flipped90);
    assert_eq!(Transform::from_hypr(7), Transform::Flipped270);
    for t in Transform::ALL {
        assert_eq!(Transform::from_hypr(t.to_hypr()), t);
    }
}

#[test]
fn odd_quarter_turns_swap_axes() {
    let swapping: Vec<_> = Transform::ALL
        .into_iter()
        .filter(|t| t.swaps_axes())
        .collect();
    assert_eq!(
        swapping,
        [
            Transform::R90,
            Transform::R270,
            Transform::Flipped90,
            Transform::Flipped270
        ]
    );
}

#[test]
fn kanshi_tokens_round_trip() {
    assert_eq!(Transform::Normal.kanshi_token(), "normal");
    assert_eq!(Transform::from_kanshi("270"), Some(Transform::R270));
    assert_eq!(
        Transform::from_kanshi("flipped-90"),
        Some(Transform::Flipped90)
    );
    for t in Transform::ALL {
        assert_eq!(Transform::from_kanshi(t.kanshi_token()), Some(t));
    }
}

#[test]
fn flip_and_rotation_compose() {
    assert_eq!(Transform::R90.with_flipped(true), Transform::Flipped90);
    assert_eq!(Transform::Flipped90.with_flipped(false), Transform::R90);
    assert_eq!(
        Transform::Flipped.with_rotation(Transform::R270),
        Transform::Flipped270
    );
    assert_eq!(Transform::Flipped270.rotation(), Transform::R270);
    assert!(Transform::Flipped.is_flipped() && !Transform::R180.is_flipped());
}
