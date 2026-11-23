use gtk4 as gtk;

use super::html;
use super::props::PaginationProps;

pub fn build(props: &PaginationProps) -> gtk::Widget {
    html::build(props)
}
