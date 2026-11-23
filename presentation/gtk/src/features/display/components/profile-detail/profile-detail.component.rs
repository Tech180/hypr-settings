use gtk4 as gtk;

use super::html;
use super::props::ProfileDetailProps;

/// The sub page of one profile: back bar, saved outputs and an apply button.
pub fn build(props: &ProfileDetailProps) -> gtk::Widget {
    html::build(props)
}
