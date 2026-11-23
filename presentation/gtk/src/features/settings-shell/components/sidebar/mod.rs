#[path = "interfaces/sidebar-props.interface.rs"]
mod props;

#[path = "sidebar.component.rs"]
mod component;
#[path = "sidebar.html.rs"]
mod html;

pub use component::build;
pub use props::{SidebarEntry, SidebarProps};
