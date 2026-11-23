#[path = "interfaces/section-header-props.interface.rs"]
mod props;

#[path = "section-header.component.rs"]
mod component;
#[path = "section-header.html.rs"]
mod html;

pub use component::build;
pub use props::{HeaderTag, HeadingSize, SectionHeaderProps};
