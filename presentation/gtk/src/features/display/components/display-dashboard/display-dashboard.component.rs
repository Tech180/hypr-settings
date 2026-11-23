use gtk4 as gtk;

use super::html;
use super::props::DisplayDashboardProps;

pub fn build(props: &DisplayDashboardProps) -> gtk::Widget {
    html::build(props)
}
