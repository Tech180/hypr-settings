use gtk::prelude::*;
use gtk4 as gtk;

/// A label over a value, e.g. `Band` / `5 GHz`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryItem {
    pub label: String,
    pub value: String,
    /// Addresses and keys read better in a monospace face.
    pub mono: bool,
}

impl TelemetryItem {
    pub fn new(label: &str, value: impl Into<String>) -> Self {
        Self {
            label: label.to_owned(),
            value: value.into(),
            mono: false,
        }
    }

    #[must_use]
    pub const fn mono(mut self) -> Self {
        self.mono = true;
        self
    }
}

const COLS: usize = 3;

pub fn build(items: &[TelemetryItem]) -> gtk::Widget {
    build_spaced(items, 16, 20)
}

pub fn build_spaced(items: &[TelemetryItem], row_spacing: u32, column_spacing: u32) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(row_spacing as i32)
        .css_classes(["details-grid", "details-grid--static"])
        .build();
    for row_items in items.chunks(COLS) {
        let row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(column_spacing as i32)
            .homogeneous(true)
            .build();
        for item in row_items {
            row.append(&cell(item));
        }
        for _ in row_items.len()..COLS {
            row.append(&placeholder_cell());
        }
        root.append(&row);
    }
    make_inert(&root);
    root.upcast()
}

fn cell(item: &TelemetryItem) -> gtk::Widget {
    let cell = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["detail-item"])
        .build();
    cell.append(
        &gtk::Label::builder()
            .label(item.label.to_uppercase())
            .xalign(0.0)
            .selectable(false)
            .focusable(false)
            .can_focus(false)
            .css_classes(["detail-item__label"])
            .build(),
    );
    let mut classes = vec!["detail-item__value"];
    if item.mono {
        classes.push("detail-item__value--mono");
    }
    cell.append(
        &gtk::Label::builder()
            .label(&item.value)
            .xalign(0.0)
            .selectable(false)
            .focusable(false)
            .can_focus(false)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::WordChar)
            .css_classes(classes)
            .build(),
    );
    make_inert(&cell);
    cell.upcast()
}

/// Keeps column alignment when the last row is short.
fn placeholder_cell() -> gtk::Widget {
    let cell = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .hexpand(true)
        .css_classes(["detail-item", "detail-item--placeholder"])
        .build();
    make_inert(&cell);
    cell.upcast()
}

/// Read-only facts: no focus ring, text selection, or pointer affordance.
fn make_inert(widget: &impl IsA<gtk::Widget>) {
    let widget = widget.as_ref();
    widget.set_focusable(false);
    widget.set_can_focus(false);
    widget.set_cursor_from_name(None);
}
