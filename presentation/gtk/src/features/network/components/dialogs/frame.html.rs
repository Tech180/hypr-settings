use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

/// The shared look of the section's dialogs: a header bar, a scrolling body and an optional
/// footer row for the main buttons.
pub fn frame(
    title: &str,
    width: i32,
    body: &gtk::Widget,
    footer: Option<&gtk::Widget>,
) -> adw::Dialog {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .css_classes(["net-dialog"])
        .build();
    content.append(body);
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .child(&content)
        .build();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&scroller));
    if let Some(footer) = footer {
        footer.add_css_class("net-dialog__footer");
        view.add_bottom_bar(footer);
    }
    adw::Dialog::builder()
        .title(title)
        .content_width(width)
        .follows_content_size(true)
        .child(&view)
        .build()
}

pub fn labelled(label: &str, field: &impl IsA<gtk::Widget>) -> gtk::Widget {
    let cell = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .hexpand(true)
        .build();
    cell.append(
        &gtk::Label::builder()
            .label(label.to_uppercase())
            .xalign(0.0)
            .css_classes(["detail-item__label"])
            .build(),
    );
    cell.append(field);
    cell.upcast()
}

pub fn entry(text: &str, placeholder: &str) -> gtk::Entry {
    let entry = gtk::Entry::builder()
        .text(text)
        .placeholder_text(placeholder)
        .hexpand(true)
        .css_classes(["net-field"])
        .build();
    entry.update_property(&[gtk::accessible::Property::Label(placeholder)]);
    entry
}

pub fn secret(text: &str, placeholder: &str) -> gtk::PasswordEntry {
    let entry = gtk::PasswordEntry::builder()
        .placeholder_text(placeholder)
        .show_peek_icon(true)
        .hexpand(true)
        .css_classes(["net-field"])
        .build();
    entry.set_text(text);
    entry
}

pub fn error_label() -> gtk::Label {
    gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .visible(false)
        .css_classes(["net-error"])
        .build()
}

pub fn show_error(label: &gtk::Label, message: &str) {
    label.set_label(message);
    label.set_visible(true);
}
