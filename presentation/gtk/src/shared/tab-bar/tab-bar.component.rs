use gtk4 as gtk;

use super::html;
use super::props::TabBarProps;

/// A row of tabs; the owner swaps the content beneath it.
pub fn build(props: &TabBarProps) -> gtk::Widget {
    html::build(props)
}
