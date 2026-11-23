#[path = "interfaces/section-search-props.interface.rs"]
mod props;

#[path = "section-search.component.rs"]
mod component;
#[path = "section-search.html.rs"]
mod html;

pub use component::build;
pub use props::SectionSearchProps;
