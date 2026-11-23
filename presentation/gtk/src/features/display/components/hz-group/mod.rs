#[path = "interfaces/hz-group-props.interface.rs"]
mod props;

#[path = "hz-group.component.rs"]
mod component;
#[path = "hz-group.html.rs"]
mod html;

pub use component::build;
pub use props::{HzGroupProps, HzRateOption};
