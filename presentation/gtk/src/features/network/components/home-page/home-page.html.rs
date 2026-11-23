use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;
use settings_domain::network::Connectivity;

use crate::features::network::components::common::{self, NavRowProps};
use crate::features::network::constants::PAGE_GAP;
use crate::features::network::interfaces::{HomeProps, HomeWidgets, NetPage, PrimaryLink};
use crate::features::network::utils::{connectivity_text, format_rate};
use crate::shared::badge::{self, Tone};
use crate::shared::button::ButtonKind;
use crate::shared::icon::{self, Icon, IconProps};
use crate::shared::icon_tile::{self, IconTileProps};

pub fn build(props: &HomeProps) -> HomeWidgets {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "network-page"])
        .build();
    let (hero, tx, rx) = hero(props);
    column.append(&hero);
    column.append(&common::section(
        "Network Connections",
        None,
        &connections(props),
    ));
    column.append(&common::section("Advanced", None, &advanced(props)));
    HomeWidgets {
        root: column.upcast(),
        tx,
        rx,
    }
}

fn hero(props: &HomeProps) -> (gtk::Widget, gtk::Label, gtk::Label) {
    let overview = &props.overview;
    let online = overview.primary != PrimaryLink::Offline;
    let hero = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(20)
        .css_classes(["boxed-group", "status-hero"])
        .build();
    let glyph = match overview.primary {
        PrimaryLink::Wifi => Icon::Wifi,
        PrimaryLink::Wired => Icon::EthernetPort,
        PrimaryLink::Offline => Icon::WifiOff,
    };
    hero.append(&icon_tile::build(&IconTileProps::large(
        glyph,
        if online { Tone::Accent } else { Tone::Neutral },
    )));

    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    text.append(
        &gtk::Label::builder()
            .label(&overview.title)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["status-hero__title"])
            .build(),
    );
    let status = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    let (tone, words) = match (online, overview.connectivity) {
        (false, _) => (Tone::Neutral, "Not connected"),
        (true, Connectivity::Limited | Connectivity::None) => {
            (Tone::Warning, connectivity_text(overview.connectivity))
        }
        (true, other) => (Tone::Accent, connectivity_text(other)),
    };
    status.append(&badge::dot(tone));
    status.append(
        &gtk::Label::builder()
            .label(words)
            .css_classes(["status-hero__subtitle"])
            .build(),
    );
    text.append(&status);
    hero.append(&text);

    let (tx_box, tx) = metric("Tx", Icon::ArrowUp);
    let (rx_box, rx) = metric("Rx", Icon::ArrowDown);
    let metrics = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(24)
        .valign(gtk::Align::Center)
        .css_classes(["net-metrics"])
        .build();
    metrics.append(&tx_box);
    metrics.append(&rx_box);
    hero.append(&metrics);
    if overview.has_wifi_profile {
        hero.append(&common::icon_action(
            "Connection properties",
            Icon::Info,
            ButtonKind::Hollow,
            props.actions.open_properties.clone(),
        ));
    }
    (hero.upcast(), tx, rx)
}

fn metric(name: &str, glyph: Icon) -> (gtk::Widget, gtk::Label) {
    let cell = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .css_classes(["net-metric"])
        .build();
    let head = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .build();
    head.append(&icon::build(&IconProps {
        icon: glyph,
        size: 12,
    }));
    head.append(
        &gtk::Label::builder()
            .label(name)
            .xalign(0.0)
            .css_classes(["net-metric__label"])
            .build(),
    );
    cell.append(&head);
    let value = gtk::Label::builder()
        .label(format_rate(None))
        .xalign(0.0)
        .valign(gtk::Align::Start)
        .css_classes(["net-metric__value"])
        .build();
    cell.append(&value);
    (cell.upcast(), value)
}

fn connections(props: &HomeProps) -> gtk::Widget {
    let overview = &props.overview;
    let actions = &props.actions;
    let open = |page: NetPage| -> Rc<dyn Fn()> {
        let open = actions.open.clone();
        Rc::new(move || open(page))
    };
    let group = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["boxed-group"])
        .overflow(gtk::Overflow::Hidden)
        .build();

    if overview.wifi_present {
        let subtitle = match (&overview.wifi_ssid, overview.wifi_enabled) {
            (Some(ssid), _) => ssid.clone(),
            (None, true) => "Not connected".to_owned(),
            (None, false) => "Off".to_owned(),
        };
        group.append(&common::nav_row(&NavRowProps {
            icon: Icon::Wifi,
            title: "Wi-Fi".into(),
            subtitle,
            subtitle_accent: overview.wifi_ssid.is_some(),
            trailing: None,
            on_open: open(NetPage::Wifi),
        }));
    }
    let ethernet = if !overview.ethernet_present {
        "No adapter"
    } else if overview.ethernet_connected {
        "Connected"
    } else {
        "Not connected"
    };
    group.append(&common::nav_row(&NavRowProps {
        icon: Icon::EthernetPort,
        title: "Ethernet".into(),
        subtitle: ethernet.into(),
        subtitle_accent: overview.ethernet_connected,
        trailing: None,
        on_open: open(NetPage::Ethernet),
    }));
    group.append(&common::nav_row(&NavRowProps {
        icon: Icon::Shield,
        title: "VPN".into(),
        subtitle: vpn_summary(overview.vpn_total, overview.vpn_active),
        subtitle_accent: overview.vpn_active > 0,
        trailing: None,
        on_open: open(NetPage::Vpn),
    }));
    group.upcast()
}

fn vpn_summary(total: usize, active: usize) -> String {
    match (total, active) {
        (0, _) => "Not configured".to_owned(),
        (_, 0) => "No active tunnels".to_owned(),
        (_, 1) => "1 active tunnel".to_owned(),
        (_, n) => format!("{n} active tunnels"),
    }
}

fn advanced(props: &HomeProps) -> gtk::Widget {
    let open = props.actions.open.clone();
    let group = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["boxed-group"])
        .overflow(gtk::Overflow::Hidden)
        .build();
    group.append(&common::nav_row(&NavRowProps {
        icon: Icon::Terminal,
        title: "Diagnostics & Logs".into(),
        subtitle: "NetworkManager tracing and status".into(),
        subtitle_accent: false,
        trailing: None,
        on_open: Rc::new(move || open(NetPage::Diagnostics)),
    }));
    group.upcast()
}

#[cfg(test)]
mod tests {
    use super::vpn_summary;

    #[test]
    fn vpn_summary_counts_tunnels() {
        assert_eq!(vpn_summary(0, 0), "Not configured");
        assert_eq!(vpn_summary(3, 0), "No active tunnels");
        assert_eq!(vpn_summary(3, 1), "1 active tunnel");
        assert_eq!(vpn_summary(3, 2), "2 active tunnels");
    }
}
