/// Distance, in canvas pixels, within which an edge or centre attracts the dragged tile.
pub const SNAP_PX: f64 = 30.0;
/// Pointer travel below which a press is a click, not a drag.
pub const CLICK_SLOP: f64 = 4.0;
pub const KEY_STEP: i32 = 10;
pub const KEY_STEP_LARGE: i32 = 100;
/// How long tiles glide to a new arrangement or a re-centred drawing.
pub const TILE_ANIMATION_MS: u32 = 200;

/// Smallest tile, in canvas pixels, that still shows each piece of text. Below these the
/// text is hidden so a tile never grows past the rectangle it stands for.
pub const LABEL_MIN_HEIGHT: f64 = 24.0;
pub const RESOLUTION_MIN_HEIGHT: f64 = 52.0;
pub const RESOLUTION_MIN_WIDTH: f64 = 56.0;
pub const BADGE_MIN_HEIGHT: f64 = 84.0;
pub const BADGE_MIN_WIDTH: f64 = 72.0;
