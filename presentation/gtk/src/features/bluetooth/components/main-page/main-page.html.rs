use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use crate::features::bluetooth::components::device_row;
use crate::features::bluetooth::constants::PAGE_GAP;
use crate::features::bluetooth::interfaces::{
    AdapterOptionProps, BluetoothActions, BluetoothMainProps, ControllerProps, DeviceRowProps,
};
use crate::shared::badge::{BadgeProps, Tone};
use crate::shared::icon::{self, Icon, IconProps};
use crate::shared::icon_tile::IconTileProps;
use crate::shared::section_header::{self, HeadingSize, SectionHeaderProps};
use crate::shared::settings_group::{self, RowControl, SettingRowProps, SettingsGroupProps};

pub fn build(props: &BluetoothMainProps) -> gtk::Widget {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "bluetooth-page"])
        .build();

    column.append(&master_toggle(props));
    if props.adapter_options.len() >= 2 {
        column.append(&adapter_picker(props));
    }
    if props.powered {
        if props.paired.is_empty() {
            column.append(&no_devices_hint());
        } else {
            column.append(&device_section("My Devices", &props.paired, &props.actions));
        }
        column.append(&other_devices_row(props));
    } else {
        column.append(&empty_state());
        if !props.paired.is_empty() {
            let remembered = device_section("My Devices", &props.paired, &props.actions);
            remembered.add_css_class("bt-content--disabled");
            remembered.set_sensitive(false);
            column.append(&remembered);
        }
    }
    column.append(&system_section(props));
    column.upcast()
}

fn adapter_picker(props: &BluetoothMainProps) -> gtk::Widget {
    let labels: Vec<String> = props
        .adapter_options
        .iter()
        .map(|o| o.label.clone())
        .collect();
    let selected = props
        .adapter_options
        .iter()
        .position(|o| {
            props
                .controller
                .as_ref()
                .is_some_and(|c| c.address.eq_ignore_ascii_case(&o.address))
        })
        .unwrap_or(0);
    let options = props.adapter_options.clone();
    let select = props.actions.select_adapter.clone();
    settings_group::build(&SettingsGroupProps {
        rows: vec![SettingRowProps {
            title: "Adapter".into(),
            subtitle: Some("Devices and scanning use this controller".into()),
            sensitive: true,
            control: RowControl::Select {
                options: labels,
                selected,
                on_select: Rc::new(move |index| {
                    if let Some(AdapterOptionProps { address, .. }) = options.get(index) {
                        select(address.clone());
                    }
                }),
            },
            leading: Some(IconTileProps::small(Icon::Cpu, Tone::Neutral)),
            badge: None,
        }],
    })
}

fn master_toggle(props: &BluetoothMainProps) -> gtk::Widget {
    let subtitle = if props.powered {
        format!("Discoverable as \"{}\"", props.alias)
    } else {
        "Off".to_owned()
    };
    let group = settings_group::build(&SettingsGroupProps {
        rows: vec![SettingRowProps {
            title: "Bluetooth".into(),
            subtitle: Some(subtitle),
            sensitive: true,
            control: RowControl::Switch {
                active: props.powered,
                on_toggle: props.actions.set_powered.clone(),
            },
            leading: Some(IconTileProps::small(
                Icon::Bluetooth,
                if props.powered {
                    Tone::Accent
                } else {
                    Tone::Neutral
                },
            )),
            badge: None,
        }],
    });
    group.add_css_class("master-toggle");
    group
}

fn empty_state() -> gtk::Widget {
    let box_ = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .halign(gtk::Align::Center)
        .css_classes(["bt-empty"])
        .build();
    box_.append(&icon::build(&IconProps {
        icon: Icon::Bluetooth,
        size: 32,
    }));
    box_.append(
        &gtk::Label::builder()
            .label(
                "Turn on Bluetooth to connect to accessories and share files with nearby devices.",
            )
            .wrap(true)
            .max_width_chars(32)
            .justify(gtk::Justification::Center)
            .css_classes(["bt-empty__text"])
            .build(),
    );
    box_.upcast()
}

fn no_devices_hint() -> gtk::Widget {
    gtk::Label::builder()
        .label("No paired devices yet. Open Other Devices to find and pair one.")
        .wrap(true)
        .max_width_chars(48)
        .css_classes(["bt-empty", "bt-empty__text"])
        .build()
        .upcast()
}

/// Scanning lives on its own page, so turning Bluetooth on never starts a scan by itself.
fn other_devices_row(props: &BluetoothMainProps) -> gtk::Widget {
    settings_group::build(&SettingsGroupProps {
        rows: vec![SettingRowProps {
            title: "Other Devices".into(),
            subtitle: Some("Scan for new accessories to pair".into()),
            sensitive: true,
            control: RowControl::Chevron(Rc::clone(&props.actions.open_scan)),
            leading: Some(IconTileProps::small(Icon::Search, Tone::Neutral)),
            badge: None,
        }],
    })
}

fn device_section(title: &str, rows: &[DeviceRowProps], actions: &BluetoothActions) -> gtk::Widget {
    let section = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    section.append(&section_header::build(&SectionHeaderProps {
        title: title.into(),
        subtitle: None,
        badge: Some(rows.len().to_string()),
        tag: None,
        size: HeadingSize::Small,
    }));
    section.append(&device_row::list(rows, actions));
    section.upcast()
}

fn system_section(props: &BluetoothMainProps) -> gtk::Widget {
    let section = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    section.append(&section_header::build(&SectionHeaderProps {
        title: "System".into(),
        subtitle: None,
        badge: None,
        tag: None,
        size: HeadingSize::Small,
    }));

    let mut rows = Vec::new();
    if let Some(controller) = &props.controller {
        rows.push(controller_row(controller, props.powered));
    }
    rows.push(SettingRowProps {
        title: "Advanced Settings".into(),
        subtitle: Some("Tethering, LE Audio and developer options".into()),
        sensitive: true,
        control: RowControl::Chevron(Rc::clone(&props.actions.open_advanced)),
        leading: Some(IconTileProps::small(Icon::Settings, Tone::Neutral)),
        badge: None,
    });
    section.append(&settings_group::build(&SettingsGroupProps { rows }));
    section.upcast()
}

fn controller_row(controller: &ControllerProps, powered: bool) -> SettingRowProps {
    SettingRowProps {
        title: "Controller".into(),
        subtitle: Some(controller.name.clone()),
        sensitive: true,
        control: RowControl::Pill(BadgeProps::new(
            controller.address.clone(),
            if powered { Tone::Accent } else { Tone::Neutral },
        )),
        leading: Some(IconTileProps::small(Icon::Cpu, Tone::Neutral)),
        badge: None,
    }
}
