use gtk::prelude::*;
use gtk4 as gtk;

use super::props::{RateChipsProps, RowControl, SettingRowProps, SettingsGroupProps};
use crate::shared::button::{self, ButtonKind, ButtonProps};
use crate::shared::dropdown::{self, DropdownProps};
use crate::shared::icon::{self, Icon, IconProps};
use crate::shared::toggle::{self, ToggleProps};
use crate::shared::{badge, icon_tile, segmented, slider};

pub fn build(props: &SettingsGroupProps) -> gtk::Widget {
    let group = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["boxed-group"])
        // Row highlights follow the rounded corners instead of poking out of them.
        .overflow(gtk::Overflow::Hidden)
        .build();
    for row in &props.rows {
        group.append(&build_row(row));
    }
    group.upcast()
}

fn build_row(props: &SettingRowProps) -> gtk::Widget {
    let title_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    title_row.append(
        &gtk::Label::builder()
            .label(&props.title)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["boxed-row__title"])
            .build(),
    );
    if let Some(props) = &props.badge {
        title_row.append(&badge::build(props));
    }

    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    text.append(&title_row);
    if let Some(subtitle) = &props.subtitle {
        text.append(
            &gtk::Label::builder()
                .label(subtitle)
                .xalign(0.0)
                .wrap(true)
                .css_classes(["boxed-row__subtitle"])
                .build(),
        );
    }

    let mut row_classes = vec!["boxed-row"];
    if matches!(props.control, RowControl::Section) {
        row_classes.push("boxed-row--section-title");
    }
    if matches!(props.control, RowControl::Chevron(_)) {
        row_classes.push("boxed-row--clickable");
    }
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .css_classes(row_classes)
        .build();
    if let Some(tile) = &props.leading {
        row.append(&icon_tile::build(tile));
    }
    row.append(&text);
    match &props.control {
        RowControl::Section => {}
        RowControl::Chevron(on_open) => attach_chevron(&row, on_open),
        other => {
            let widget = control(other);
            if matches!(other, RowControl::Slider(_)) {
                widget.set_hexpand(true);
                widget.set_halign(gtk::Align::Fill);
            }
            row.append(&widget);
        }
    }
    row.set_sensitive(props.sensitive);
    row.upcast()
}

fn attach_chevron(row: &gtk::Box, on_open: &std::rc::Rc<dyn Fn()>) {
    let chevron = icon::build(&IconProps {
        icon: Icon::ChevronRight,
        size: 18,
    });
    chevron.add_css_class("boxed-row__chevron");
    row.append(&chevron);
    row.set_cursor_from_name(Some("pointer"));
    let on_open = on_open.clone();
    let gesture = gtk::GestureClick::new();
    gesture.connect_released(move |_, _, _, _| on_open());
    row.add_controller(gesture);
}

fn control(control: &RowControl) -> gtk::Widget {
    match control {
        RowControl::Switch { active, on_toggle } => toggle::build(&ToggleProps {
            active: *active,
            on_toggle: on_toggle.clone(),
        }),
        RowControl::Select {
            options,
            selected,
            on_select,
        } => dropdown::build(&DropdownProps {
            options: options.clone(),
            selected: *selected,
            on_select: on_select.clone(),
        }),
        RowControl::Segmented(props) => segmented::build(props),
        RowControl::Slider(props) => slider::build(props),
        RowControl::Button { label, on_click } => button::build(&ButtonProps::new(
            label,
            ButtonKind::Ghost,
            on_click.clone(),
        ))
        .upcast(),
        RowControl::RateChips(props) => rate_chips(props),
        RowControl::Pill(props) => badge::build(props),
        RowControl::Chevron(_) | RowControl::Section => {
            gtk::Box::new(gtk::Orientation::Horizontal, 0).upcast()
        }
    }
}

fn rate_chips(props: &RateChipsProps) -> gtk::Widget {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .halign(gtk::Align::End)
        .valign(gtk::Align::Center)
        .build();
    for &rate in &props.rates {
        let chip = gtk::Button::builder()
            .label(format!("{rate} Hz ✕"))
            .tooltip_text("Remove this rate")
            .css_classes(["rate-chip"])
            .build();
        let on_remove = props.on_remove.clone();
        chip.connect_clicked(move |_| on_remove(rate));
        row.append(&chip);
    }
    let entry = gtk::Entry::builder()
        .placeholder_text("Hz")
        .max_width_chars(5)
        .width_chars(5)
        .input_purpose(gtk::InputPurpose::Digits)
        .css_classes(["rate-entry"])
        .build();
    let add = gtk::Button::builder()
        .label("Custom +")
        .css_classes(["action-btn"])
        .build();
    let submit = {
        let (entry, on_add) = (entry.clone(), props.on_add.clone());
        move || {
            if let Ok(hz) = entry.text().trim().parse::<u32>() {
                entry.set_text("");
                on_add(hz);
            }
        }
    };
    entry.connect_activate({
        let submit = submit.clone();
        move |_| submit()
    });
    add.connect_clicked(move |_| submit());
    row.append(&entry);
    row.append(&add);
    row.upcast()
}
