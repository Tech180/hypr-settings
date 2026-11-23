use crate::value::{Resolution, Scale, Transform};

/// Hyprland's rule: `round(pixels / scale)`, axes swapped for 90 and 270 degrees.
pub fn logical_size(resolution: Resolution, scale: Scale, transform: Transform) -> (i32, i32) {
    let (w, h) = if transform.swaps_axes() {
        (resolution.height, resolution.width)
    } else {
        (resolution.width, resolution.height)
    };
    let s = i64::from(scale.hundredths()).max(1);
    let divide = |px: u32| ((i64::from(px) * 100 + s / 2) / s) as i32;
    (divide(w), divide(h))
}
