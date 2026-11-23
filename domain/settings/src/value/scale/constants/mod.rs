#[path = "scale.constants.rs"]
mod bounds;
#[path = "scale.presets.constant.rs"]
mod presets;

pub(crate) use bounds::{ALL_VALID_MAX_HUNDREDTHS, COMMON_SCALES, MAX_HUNDREDTHS, MIN_HUNDREDTHS};
pub use presets::SCALE_PRESETS;
