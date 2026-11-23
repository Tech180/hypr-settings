//! Default scale chips when no resolution is known yet.

use crate::value::scale::interfaces::Scale;

pub const SCALE_PRESETS: [Scale; 5] = [
    Scale::raw(100),
    Scale::raw(125),
    Scale::raw(150),
    Scale::raw(175),
    Scale::raw(200),
];
