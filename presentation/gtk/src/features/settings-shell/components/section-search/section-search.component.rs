use gtk4 as gtk;

use super::html;
use super::props::SectionSearchProps;

pub fn build(props: &SectionSearchProps) -> gtk::SearchEntry {
    html::build(props)
}
