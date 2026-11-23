use serde::{Deserialize, Serialize};

/// The eight wl_output transforms Hyprland and kanshi share: four rotations, each
/// optionally mirrored. Hyprland integers 0-7 map one to one onto the variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Transform {
    Normal,
    R90,
    R180,
    R270,
    Flipped,
    Flipped90,
    Flipped180,
    Flipped270,
}
