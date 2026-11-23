use gtk::prelude::*;
use gtk4 as gtk;

use crate::features::bluetooth::constants::PAGE_GAP;
use crate::features::bluetooth::interfaces::AdvancedPageProps;
use crate::features::bluetooth::utils::{auracast_row, developer_rows, pan_row};
use crate::shared::badge::{self, BadgeProps, Tone};
use crate::shared::icon::Icon;
use crate::shared::icon_tile::{self, IconTileProps};
use crate::shared::settings_group::{self, SettingsGroupProps};
use crate::shared::subpage_bar;
use crate::shared::tab_bar::{self, TabBarProps, TabProps};

pub const TAB_CONNECTIVITY: usize = 0;

pub fn build(props: &AdvancedPageProps) -> gtk::Widget {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "page-content--sub", "bluetooth-page"])
        .build();
    column.append(&controller_card(props));
    column.append(&tab_bar::build(&TabBarProps {
        tabs: vec![
            TabProps {
                label: "Connectivity".into(),
                icon: Some(Icon::Globe),
            },
            TabProps {
                label: "Developer".into(),
                icon: Some(Icon::Terminal),
            },
        ],
        selected: props.tab,
        on_select: props.actions.select_advanced_tab.clone(),
    }));
    if props.tab == TAB_CONNECTIVITY {
        column.append(&connectivity(props));
    } else {
        column.append(&developer(props));
    }
    subpage_bar::frame(
        "Advanced Settings",
        props.actions.close_subpage.clone(),
        column.upcast_ref(),
    )
}

/// The adapter at a glance: what it is, where it lives, and whether it is on.
fn controller_card(props: &AdvancedPageProps) -> gtk::Widget {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(20)
        .css_classes(["boxed-group", "bt-controller"])
        .build();
    card.append(&icon_tile::build(&IconTileProps::large(
        Icon::Cpu,
        if props.powered {
            Tone::Accent
        } else {
            Tone::Neutral
        },
    )));

    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    let title_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    title_row.append(
        &gtk::Label::builder()
            .label(&props.adapter_name)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["bt-controller__name"])
            .build(),
    );
    if props.multi_adapter && !props.adapter_address.is_empty() {
        title_row.append(&badge::build(&BadgeProps::new(
            props.adapter_address.clone(),
            Tone::Neutral,
        )));
    }
    text.append(&title_row);
    if !props.adapter_address.is_empty() {
        text.append(
            &gtk::Label::builder()
                .label(&props.adapter_address)
                .xalign(0.0)
                .selectable(true)
                .css_classes(["bt-controller__address"])
                .build(),
        );
    }
    if let Some(lmp) = &props.lmp {
        let pills = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(4)
            .build();
        pills.append(&badge::build(&BadgeProps::new(
            format!("Bluetooth {lmp}"),
            Tone::Neutral,
        )));
        text.append(&pills);
    }
    card.append(&text);
    card.upcast()
}

fn connectivity(props: &AdvancedPageProps) -> gtk::Widget {
    settings_group::build(&SettingsGroupProps {
        rows: vec![pan_row(props), auracast_row(props)],
    })
}

fn developer(props: &AdvancedPageProps) -> gtk::Widget {
    let section = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP / 2)
        .build();
    section.append(&warning_banner());
    let group = settings_group::build(&SettingsGroupProps {
        rows: developer_rows(props),
    });
    group.add_css_class("bt-developer-group");
    section.append(&group);
    section.upcast()
}

fn warning_banner() -> gtk::Widget {
    let banner = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(14)
        .css_classes(["bt-banner"])
        .build();
    banner.append(&icon_tile::build(&IconTileProps::small(
        Icon::TriangleAlert,
        Tone::Warning,
    )));
    banner.append(
        &gtk::Label::builder()
            .label("These options apply to every Bluetooth adapter on this system. They change how BlueZ and PipeWire behave and may require restarting Bluetooth.")
            .xalign(0.0)
            .wrap(true)
            .hexpand(true)
            .valign(gtk::Align::Center)
            .css_classes(["bt-banner__text"])
            .build(),
    );
    banner.upcast()
}
