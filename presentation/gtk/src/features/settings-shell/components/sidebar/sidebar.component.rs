use super::html;
use super::props::{SidebarProps, SidebarWidgets};

pub fn build(props: &SidebarProps) -> SidebarWidgets {
    html::build(props)
}
