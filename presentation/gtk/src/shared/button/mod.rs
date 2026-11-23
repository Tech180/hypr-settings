#[path = "interfaces/button-props.interface.rs"]
mod props;

#[path = "button.component.rs"]
mod component;
#[path = "button.html.rs"]
mod html;

pub use component::build;
pub use props::{ButtonKind, ButtonProps, ButtonSize};
