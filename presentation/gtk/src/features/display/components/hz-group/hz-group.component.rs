use gtk4 as gtk;

use super::html;
use super::props::HzGroupProps;

pub fn build(props: &HzGroupProps) -> gtk::Widget {
    html::build(props)
}
