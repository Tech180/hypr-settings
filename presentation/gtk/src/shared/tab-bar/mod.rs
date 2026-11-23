#[path = "interfaces/tab-bar-props.interface.rs"]
mod props;

#[path = "tab-bar.component.rs"]
mod component;
#[path = "tab-bar.html.rs"]
mod html;

pub use component::build;
pub use props::{TabBarProps, TabProps};
