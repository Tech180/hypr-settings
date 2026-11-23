use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use crate::features::bluetooth::components::device_row;
use crate::features::bluetooth::constants::PAGE_GAP;
use crate::features::bluetooth::interfaces::BluetoothScanProps;
use crate::features::bluetooth::utils::filter_devices;
use crate::shared::badge::Tone;
use crate::shared::button::{self, ButtonKind, ButtonProps, ButtonSize};
use crate::shared::icon::Icon;
use crate::shared::icon_tile::{self, IconTileProps};
use crate::shared::search_field::{self, SearchFieldProps};
use crate::shared::section_header::{self, HeadingSize, SectionHeaderProps};
use crate::shared::subpage_bar;

pub fn build(props: &BluetoothScanProps) -> gtk::Widget {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "page-content--sub", "bluetooth-page"])
        .build();
    column.append(&scan_card(props));
    column.append(&find_section(props));
    subpage_bar::frame(
        "Other Devices",
        props.actions.close_subpage.clone(),
        column.upcast_ref(),
    )
}

/// What the radio is doing, and the one button that changes it.
fn scan_card(props: &BluetoothScanProps) -> gtk::Widget {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(20)
        .css_classes(["boxed-group", "bt-controller"])
        .build();
    card.append(&icon_tile::build(&IconTileProps::large(
        Icon::Search,
        if props.discovering {
            Tone::Accent
        } else {
            Tone::Neutral
        },
    )));

    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    text.append(
        &gtk::Label::builder()
            .label(if props.discovering {
                "Looking for devices"
            } else {
                "Scan is off"
            })
            .xalign(0.0)
            .css_classes(["bt-controller__name"])
            .build(),
    );
    text.append(
        &gtk::Label::builder()
            .label(if props.discovering {
                "Put your device in pairing mode. Scanning stops when you leave this page."
            } else {
                "Start a scan to see accessories that are ready to pair."
            })
            .xalign(0.0)
            .wrap(true)
            .css_classes(["bt-empty__text"])
            .build(),
    );
    card.append(&text);

    if props.discovering {
        let spinner = gtk::Spinner::builder().valign(gtk::Align::Center).build();
        spinner.start();
        card.append(&spinner);
    }
    let (label, kind, action) = if props.discovering {
        (
            "Stop",
            ButtonKind::Hollow,
            Rc::clone(&props.actions.stop_scan),
        )
    } else {
        (
            "Scan",
            ButtonKind::Primary,
            Rc::clone(&props.actions.start_scan),
        )
    };
    card.append(
        &button::build(&ButtonProps::new(label, kind, action).size(ButtonSize::Regular))
            .upcast::<gtk::Widget>(),
    );
    card.upcast()
}

/// A search field over the results. Typing refilters the results in place, so the field
/// keeps its focus and cursor.
fn find_section(props: &BluetoothScanProps) -> gtk::Widget {
    let section = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .build();
    let results = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    fill_results(&results, props, &props.query);

    if !props.devices.is_empty() || !props.query.is_empty() {
        let on_query = props.on_query.clone();
        let (slot, shown) = (results.clone(), props.clone());
        section.append(&search_field::build(&SearchFieldProps {
            on_change: Rc::new(move |query| {
                on_query(query.clone());
                fill_results(&slot, &shown, &query);
            }),
            ..SearchFieldProps::new("Search by name or address", Rc::new(|_| {})).text(&props.query)
        }));
    }
    section.append(&results);
    section.upcast()
}

fn fill_results(slot: &gtk::Box, props: &BluetoothScanProps, query: &str) {
    while let Some(child) = slot.first_child() {
        slot.remove(&child);
    }
    let shown = filter_devices(&props.devices, query);
    if shown.is_empty() {
        if !props.devices.is_empty() {
            slot.append(&hint(&format!("No devices match \"{}\".", query.trim())));
        } else if props.discovering {
            slot.append(&hint(
                "Nothing found yet. New devices appear here as they are seen.",
            ));
        }
        return;
    }
    slot.append(&section_header::build(&SectionHeaderProps {
        title: "Available".into(),
        subtitle: None,
        badge: Some(shown.len().to_string()),
        tag: None,
        size: HeadingSize::Small,
    }));
    slot.append(&device_row::list(&shown, &props.actions));
}

fn hint(text: &str) -> gtk::Widget {
    gtk::Label::builder()
        .label(text)
        .wrap(true)
        .max_width_chars(48)
        .css_classes(["bt-empty", "bt-empty__text"])
        .build()
        .upcast()
}
