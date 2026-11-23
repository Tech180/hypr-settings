use gtk::prelude::*;
use gtk4 as gtk;

use super::props::SectionSearchProps;
use crate::shared::search_field::{self, SearchFieldProps};

pub fn build(props: &SectionSearchProps) -> gtk::SearchEntry {
    let entry = search_field::build(&SearchFieldProps {
        on_activate: Some(props.on_activate.clone()),
        on_dismiss: Some(props.on_dismiss.clone()),
        ..SearchFieldProps::new("Search settings", props.on_change.clone())
    });
    entry.add_css_class("section-search");
    entry
}
