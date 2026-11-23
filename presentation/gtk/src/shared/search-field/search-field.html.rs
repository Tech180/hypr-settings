use gtk::prelude::*;
use gtk4 as gtk;

use super::props::SearchFieldProps;

pub fn build(props: &SearchFieldProps) -> gtk::SearchEntry {
    let entry = gtk::SearchEntry::builder()
        .placeholder_text(&props.placeholder)
        .text(&props.text)
        .hexpand(true)
        .css_classes(["search-field"])
        .build();

    let on_change = props.on_change.clone();
    entry.connect_search_changed(move |entry| on_change(entry.text().to_string()));

    if let Some(on_activate) = props.on_activate.clone() {
        entry.connect_activate(move |_| on_activate());
    }

    let on_dismiss = props.on_dismiss.clone();
    entry.connect_stop_search(move |entry| match &on_dismiss {
        Some(dismiss) if entry.text().is_empty() => dismiss(),
        _ => entry.set_text(""),
    });
    entry
}
