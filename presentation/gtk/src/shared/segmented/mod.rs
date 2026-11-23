#[path = "interfaces/segmented-props.interface.rs"]
mod props;

#[path = "segmented.component.rs"]
mod component;
#[path = "segmented.html.rs"]
mod html;

pub use component::build;
pub use props::SegmentedProps;
