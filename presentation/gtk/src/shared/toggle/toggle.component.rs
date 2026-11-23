use gtk4 as gtk;

use super::html;
use super::props::ToggleProps;

/// The on/off switch every settings row shares.
pub fn build(props: &ToggleProps) -> gtk::Widget {
    html::build(props)
}
