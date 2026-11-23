use crate::value::transform::interfaces::Transform;

pub const ALL: [Transform; 8] = [
    Transform::Normal,
    Transform::R90,
    Transform::R180,
    Transform::R270,
    Transform::Flipped,
    Transform::Flipped90,
    Transform::Flipped180,
    Transform::Flipped270,
];

/// The unmirrored rotations, in the order the UI offers them.
pub const ROTATIONS: [Transform; 4] = [
    Transform::Normal,
    Transform::R90,
    Transform::R180,
    Transform::R270,
];
