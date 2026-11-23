use gtk4 as gtk;

use super::html;
use super::props::SelectProps;

pub fn build(props: &SelectProps) -> gtk::Widget {
    html::build(props)
}
