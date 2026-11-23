#[path = "canvas-geometry.util.rs"]
pub mod canvas_geometry;
#[path = "format-mode.util.rs"]
mod format_mode;
#[path = "hardware-props.util.rs"]
mod hardware_props;
#[path = "page-bind.util.rs"]
pub mod page_bind;
#[path = "page-props.util.rs"]
mod page_props;

pub use format_mode::{coordinates, trial_message};
pub use hardware_props::Hardware;
pub use page_bind::bind;
pub use page_props::{arrangement_header, page_props, profile_detail_props, resolve_selection};
