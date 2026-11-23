#[path = "interfaces/dropdown-props.interface.rs"]
mod props;

#[path = "dropdown.component.rs"]
mod component;
#[path = "dropdown.html.rs"]
mod html;

pub use component::build;
pub use props::DropdownProps;
