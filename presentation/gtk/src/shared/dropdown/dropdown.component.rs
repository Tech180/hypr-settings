use gtk4 as gtk;

use super::html;
use super::props::DropdownProps;

/// A single-choice menu that opens beneath its button.
pub fn build(props: &DropdownProps) -> gtk::Widget {
    html::build(props)
}
