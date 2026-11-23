use gtk::prelude::*;
use gtk4 as gtk;
use settings_domain::bluetooth::DeviceClass;
use std::rc::Rc;

use crate::features::bluetooth::components::device_row::battery_tone;
use crate::features::bluetooth::constants::PAGE_GAP;
use crate::features::bluetooth::interfaces::DeviceDetailProps;
use crate::features::bluetooth::utils::{present_forget_dialog, profile_rows};
use crate::shared::badge::{self, BadgeProps, Tone};
use crate::shared::button::{self, ButtonKind, ButtonProps, ButtonSize};
use crate::shared::icon::Icon;
use crate::shared::icon_tile::{self, IconTileProps};
use crate::shared::section_header::{self, HeadingSize, SectionHeaderProps};
use crate::shared::settings_group::{self, RowControl, SettingRowProps, SettingsGroupProps};
use crate::shared::subpage_bar;

pub fn build(props: &DeviceDetailProps) -> gtk::Widget {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "page-content--sub", "bluetooth-page"])
        .build();
    column.append(&hero(props));

    let id = props.device.id.clone();
    let mut rows = vec![
        SettingRowProps {
            title: "Connection".into(),
            subtitle: Some("Toggle active link to device".into()),
            sensitive: true,
            control: {
                let set = props.actions.set_connected.clone();
                let id = id.clone();
                RowControl::Switch {
                    active: props.device.connected,
                    on_toggle: Rc::new(move |on| set(id.clone(), on)),
                }
            },
            leading: None,
            badge: None,
        },
        type_row(props),
    ];
    if props.device.class == DeviceClass::Headphones
        && let Some(codec) = &props.device.codec
    {
        let labels = codec.options.clone();
        let selected = labels.iter().position(|o| o == &codec.active).unwrap_or(0);
        let set_codec = props.actions.set_codec.clone();
        let id = props.device.id.clone();
        rows.push(SettingRowProps {
            title: "Audio Codec".into(),
            subtitle: Some("LDAC offers highest quality".into()),
            sensitive: props.capabilities.codec_switch,
            control: RowControl::Select {
                options: labels.clone(),
                selected,
                on_select: Rc::new(move |index| {
                    if let Some(label) = labels.get(index) {
                        set_codec(id.clone(), label.clone());
                    }
                }),
            },
            leading: None,
            badge: (!props.capabilities.codec_switch)
                .then(|| BadgeProps::new("Unavailable", Tone::Neutral)),
        });
    }
    column.append(&settings_group::build(&SettingsGroupProps { rows }));

    let profiles = profile_rows(props);
    if !profiles.is_empty() {
        column.append(&section_title("Profiles"));
        column.append(&settings_group::build(&SettingsGroupProps {
            rows: profiles,
        }));
    }

    let forget = props.actions.forget.clone();
    let device_id = props.device.id.clone();
    column.append(
        &button::build(
            &ButtonProps::new(
                "Forget This Device",
                ButtonKind::Danger,
                Rc::new(move || present_forget_dialog(&device_id, forget.clone())),
            )
            .size(ButtonSize::Large)
            .icon(Icon::Trash),
        )
        .upcast::<gtk::Widget>(),
    );
    subpage_bar::frame(
        "Device Settings",
        props.actions.close_subpage.clone(),
        column.upcast_ref(),
    )
}

fn section_title(title: &str) -> gtk::Widget {
    section_header::build(&SectionHeaderProps {
        title: title.into(),
        subtitle: None,
        badge: None,
        tag: None,
        size: HeadingSize::Small,
    })
    .upcast()
}

fn hero(props: &DeviceDetailProps) -> gtk::Widget {
    let device = &props.device;
    let hero = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .css_classes(["device-hero"])
        .build();
    hero.append(&icon_tile::build(&IconTileProps::large(
        device_icon(device.class),
        if device.connected {
            Tone::Accent
        } else {
            Tone::Neutral
        },
    )));
    hero.append(
        &gtk::Label::builder()
            .label(&device.name)
            .wrap(true)
            .max_width_chars(36)
            .justify(gtk::Justification::Center)
            .css_classes(["device-hero__title"])
            .build(),
    );

    let status = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::Center)
        .build();
    if device.connected {
        status.append(&badge::dot(Tone::Accent));
    }
    status.append(
        &gtk::Label::builder()
            .label(if device.connected {
                "Connected"
            } else {
                "Not connected"
            })
            .css_classes(["device-hero__status"])
            .build(),
    );
    if let Some(pct) = device.battery_percent {
        status.append(&badge::build(&BadgeProps::new(
            format!("Battery {pct}%"),
            battery_tone(pct),
        )));
    }
    hero.append(&status);
    hero.upcast()
}

fn device_icon(class: DeviceClass) -> Icon {
    match class {
        DeviceClass::Headphones => Icon::Headphones,
        DeviceClass::Keyboard => Icon::Keyboard,
        DeviceClass::Mouse => Icon::Mouse,
        DeviceClass::Phone => Icon::Smartphone,
        DeviceClass::Gamepad => Icon::Gamepad2,
        DeviceClass::Other => Icon::Bluetooth,
    }
}

fn type_row(props: &DeviceDetailProps) -> SettingRowProps {
    let classes = props.class_labels.clone();
    let labels: Vec<String> = classes.iter().map(|(_, l)| l.clone()).collect();
    let selected = classes
        .iter()
        .position(|(c, _)| *c == props.device.class)
        .unwrap_or(0);
    let set_class = props.actions.set_class.clone();
    let id = props.device.id.clone();
    SettingRowProps {
        title: "Device Type".into(),
        subtitle: None,
        sensitive: true,
        control: RowControl::Select {
            options: labels,
            selected,
            on_select: Rc::new(move |index| {
                if let Some((class, _)) = classes.get(index) {
                    set_class(id.clone(), *class);
                }
            }),
        },
        leading: None,
        badge: None,
    }
}
