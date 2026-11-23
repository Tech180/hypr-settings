use std::rc::Rc;

use gtk4 as gtk;
use libadwaita as adw;

use crate::shared::icon::Icon;

pub struct SidebarEntry {
    pub id: String,
    pub title: String,
    pub icon: Icon,
    /// Rows that are not wired up yet are drawn dimmed and cannot be selected.
    pub enabled: bool,
}

pub struct SidebarProps {
    pub title: String,
    pub entries: Vec<SidebarEntry>,
    pub search: gtk::Widget,
    pub on_select: Rc<dyn Fn(String)>,
}

pub struct SidebarWidgets {
    pub page: adw::NavigationPage,
    pub list: gtk::ListBox,
}
