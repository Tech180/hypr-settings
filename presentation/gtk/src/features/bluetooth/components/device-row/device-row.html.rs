use gtk::prelude::*;
use gtk4 as gtk;

use crate::features::bluetooth::interfaces::{BluetoothActions, DeviceRowProps};
use crate::shared::badge::{self, BadgeProps, Tone};
use crate::shared::button::{self, ButtonKind, ButtonProps, ButtonSize};
use crate::shared::icon::{self, Icon, IconProps};
use crate::shared::icon_tile::{self, IconTileProps};

/// One device in a list: tile, name and state, battery, quick action, and a chevron that
/// opens its settings. The whole row is clickable.
pub fn build(row: &DeviceRowProps, actions: &BluetoothActions) -> gtk::Widget {
    let connected = row.device.connected;
    let mut classes = vec!["boxed-row", "boxed-row--clickable", "device-row"];
    if connected {
        classes.push("device-row--connected");
    }
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(14)
        .css_classes(classes)
        .build();
    root.append(&icon_tile::build(&IconTileProps::small(
        row.icon,
        if connected {
            Tone::Accent
        } else {
            Tone::Neutral
        },
    )));
    root.append(&text(row));

    if let Some(pct) = row.device.battery_percent {
        root.append(&badge::build(&BadgeProps::new(
            format!("{pct}%"),
            battery_tone(pct),
        )));
    }
    if row.show_connect {
        root.append(&quick_action(row, actions));
    }
    let chevron = icon::build(&IconProps {
        icon: Icon::ChevronRight,
        size: 18,
    });
    chevron.add_css_class("boxed-row__chevron");
    root.append(&chevron);

    root.set_cursor_from_name(Some("pointer"));
    let open = actions.open_device.clone();
    let id = row.device.id.clone();
    let gesture = gtk::GestureClick::new();
    gesture.connect_released(move |_, _, _, _| open(id.clone()));
    root.add_controller(gesture);
    root.upcast()
}

fn text(row: &DeviceRowProps) -> gtk::Widget {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    column.append(
        &gtk::Label::builder()
            .label(&row.device.name)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["boxed-row__title"])
            .build(),
    );

    let status = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .build();
    let label = if row.device.connected {
        status.append(&badge::dot(Tone::Accent));
        "Connected"
    } else if row.device.paired {
        "Paired"
    } else {
        "Available to pair"
    };
    status.append(
        &gtk::Label::builder()
            .label(label)
            .xalign(0.0)
            .css_classes(["boxed-row__subtitle"])
            .build(),
    );
    column.append(&status);
    column.upcast()
}

fn quick_action(row: &DeviceRowProps, actions: &BluetoothActions) -> gtk::Widget {
    let id = row.device.id.clone();
    let paired = row.device.paired;
    let (connect, pair) = (actions.connect.clone(), actions.pair.clone());
    let label = if paired { "Connect" } else { "Pair" };
    button::build(
        &ButtonProps::new(
            label,
            ButtonKind::Hollow,
            std::rc::Rc::new(move || {
                if paired {
                    connect(id.clone());
                } else {
                    pair(id.clone());
                }
            }),
        )
        .size(ButtonSize::Compact),
    )
    .upcast()
}

pub fn battery_tone(percent: u8) -> Tone {
    match percent {
        0..=10 => Tone::Danger,
        11..=20 => Tone::Warning,
        _ => Tone::Neutral,
    }
}

/// A card holding one row per device.
pub fn list(rows: &[DeviceRowProps], actions: &BluetoothActions) -> gtk::Widget {
    let group = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["boxed-group"])
        .overflow(gtk::Overflow::Hidden)
        .build();
    for row in rows {
        group.append(&build(row, actions));
    }
    group.upcast()
}
