#[path = "interfaces/search-field-props.interface.rs"]
mod props;

#[path = "search-field.component.rs"]
mod component;
#[path = "search-field.html.rs"]
mod html;

pub use component::build;
pub use props::SearchFieldProps;
