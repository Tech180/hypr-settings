use gtk4 as gtk;

use super::html;
use super::props::SettingsGroupProps;

/// One card holding a labelled row per setting, each with its own control.
pub fn build(props: &SettingsGroupProps) -> gtk::Widget {
    html::build(props)
}
