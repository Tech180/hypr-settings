use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

#[derive(Clone)]
pub struct PasswordFieldProps {
    pub placeholder: String,
    pub text: String,
    pub on_change: Rc<dyn Fn(String)>,
    /// Enter pressed.
    pub on_submit: Rc<dyn Fn()>,
}

/// A masked field with a peek icon; what the user typed survives page rebuilds because
/// the page hands it back as `text`.
pub fn build(props: &PasswordFieldProps) -> gtk::PasswordEntry {
    let entry = gtk::PasswordEntry::builder()
        .placeholder_text(&props.placeholder)
        .show_peek_icon(true)
        .hexpand(true)
        .css_classes(["net-field"])
        .build();
    entry.set_text(&props.text);
    entry.update_property(&[gtk::accessible::Property::Label("Network password")]);
    let on_change = props.on_change.clone();
    entry.connect_changed(move |entry| on_change(entry.text().to_string()));
    let on_submit = props.on_submit.clone();
    entry.connect_activate(move |_| on_submit());
    entry
}
