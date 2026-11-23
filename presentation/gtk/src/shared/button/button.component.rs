use gtk4 as gtk;

use super::html;
use super::props::ButtonProps;

/// A push button in one of the shared looks (ghost, hollow, primary, danger).
pub fn build(props: &ButtonProps) -> gtk::Button {
    html::build(props)
}
