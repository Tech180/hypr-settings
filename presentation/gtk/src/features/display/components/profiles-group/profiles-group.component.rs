use gtk4 as gtk;

use super::html;
use super::props::ProfilesGroupProps;

pub fn build(props: &ProfilesGroupProps) -> gtk::Widget {
    html::build(props)
}
