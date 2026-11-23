use std::rc::Rc;

use gtk4 as gtk;

use crate::shared::icon::Icon;

/// A row that opens to show details; the header stays visible.
#[derive(Clone)]
pub struct CardProps {
    pub title: String,
    pub subtitle: String,
    /// A lock after the subtitle for secured networks.
    pub locked: bool,
    pub icon: Icon,
    /// The connected or running one: accent border, filled tile.
    pub active: bool,
    pub expanded: bool,
    /// Buttons and badges between the text and the chevron.
    pub meta: Vec<gtk::Widget>,
    pub details: gtk::Widget,
    pub on_toggle: Rc<dyn Fn(bool)>,
}
