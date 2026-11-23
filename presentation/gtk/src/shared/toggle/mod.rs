#[path = "interfaces/toggle-props.interface.rs"]
mod props;

#[path = "toggle.component.rs"]
mod component;
#[path = "toggle.html.rs"]
mod html;

pub use component::build;
pub use props::ToggleProps;
