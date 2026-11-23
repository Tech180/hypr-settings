use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;
use settings_domain::network::{LinkState, NetworkDevice};

use crate::features::network::components::common;
use crate::features::network::components::ip_settings::{self, IpSettingsProps};
use crate::features::network::components::expander_card::{self, CardProps};
use crate::features::network::components::telemetry_grid::{self, TelemetryItem};
use crate::features::network::constants::PAGE_GAP;
use crate::features::network::interfaces::EthernetProps;
use crate::features::network::utils::{
    device_label, dns_text, format_bytes, link_text, or_dash,
};
use crate::shared::dropdown::{self, DropdownProps};
use crate::shared::icon::Icon;
use crate::shared::subpage_bar;
use crate::shared::toggle::{self, ToggleProps};

const COLLAPSED_KEY: &str = "ethernet-collapsed";

pub fn build(props: &EthernetProps) -> gtk::Widget {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "page-content--sub", "network-page"])
        .build();
    match &props.view.device {
        None => column.append(&common::empty_state(
            Icon::EthernetPort,
            "No Ethernet adapter",
            "Plug in a USB or dock adapter and it appears here.",
        )),
        Some(device) => {
            column.append(&toolbar(props, device));
            column.append(&status(props, device));
            if device.connected {
                column.append(&ipv4(props));
            }
        }
    }
    subpage_bar::frame("Ethernet", props.actions.back.clone(), column.upcast_ref())
}

fn toolbar(props: &EthernetProps, device: &NetworkDevice) -> gtk::Widget {
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .valign(gtk::Align::Center)
        .css_classes(["net-toolbar"])
        .build();
    let set = props.actions.set_ethernet_enabled.clone();
    let switch = toggle::build(&ToggleProps {
        active: device.enabled,
        on_toggle: Rc::new(move |on| set(on)),
    });
    switch.set_valign(gtk::Align::Center);
    bar.append(&switch);
    if !props.view.devices.is_empty() {
        let ifaces: Vec<String> = props.view.devices.iter().map(|d| d.iface.clone()).collect();
        let selected = ifaces
            .iter()
            .position(|iface| *iface == props.view.selected)
            .unwrap_or(0);
        let select = props.actions.select_ethernet_adapter.clone();
        bar.append(&dropdown::build(&DropdownProps {
            options: props.view.devices.iter().map(device_label).collect(),
            selected,
            on_select: Rc::new(move |index| {
                if let Some(iface) = ifaces.get(index) {
                    select(iface.clone());
                }
            }),
        }));
    }
    bar.upcast()
}

fn status(props: &EthernetProps, device: &NetworkDevice) -> gtk::Widget {
    let state = device.link_state();
    let toggled = props.actions.card_toggled.clone();
    let card = expander_card::build(&CardProps {
        title: device_label(device),
        subtitle: link_text(state, device.kind).to_owned(),
        locked: false,
        icon: Icon::EthernetPort,
        active: state == LinkState::Connected,
        expanded: !props.memory.expanded.contains(COLLAPSED_KEY),
        meta: Vec::new(),
        details: details(device),
        on_toggle: Rc::new(move |open| toggled(COLLAPSED_KEY.to_owned(), !open)),
    });
    common::section(
        "Status",
        None,
        common::card_list(std::slice::from_ref(&card)).upcast_ref(),
    )
}

fn details(device: &NetworkDevice) -> gtk::Widget {
    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .css_classes(["net-card__body"])
        .build();
    let speed = device
        .speed_mbps
        .map_or_else(|| "—".to_owned(), |mbps| format!("{mbps} Mbps"));
    let bytes = |value: Option<u64>| value.map_or_else(|| "—".to_owned(), format_bytes);
    body.append(&telemetry_grid::build(&[
        TelemetryItem::new("Link rate", speed),
        TelemetryItem::new("IP address", or_dash(&device.ip)).mono(),
        TelemetryItem::new("Gateway", or_dash(&device.gateway)).mono(),
        TelemetryItem::new("DNS", dns_text(&device.dns)).mono(),
        TelemetryItem::new("IPv6", or_dash(&device.ipv6)).mono(),
        TelemetryItem::new("MAC", or_dash(&device.mac)).mono(),
        TelemetryItem::new(
            "MTU",
            device
                .mtu
                .map_or_else(|| "—".to_owned(), |mtu| mtu.to_string()),
        ),
        TelemetryItem::new("Received", bytes(device.rx_bytes)),
        TelemetryItem::new("Sent", bytes(device.tx_bytes)),
        TelemetryItem::new("Connection", or_dash(&device.connection_name)),
    ]));
    body.upcast()
}

fn ipv4(props: &EthernetProps) -> gtk::Widget {
    let Some(profile) = &props.view.profile else {
        return common::section(
            "IP settings",
            None,
            &common::hint("This link has no saved connection to configure."),
        );
    };
    let apply = props.apply_ip.clone();
    common::section(
        "IP settings",
        None,
        &ip_settings::build(&IpSettingsProps {
            ipv4: profile.ipv4.clone(),
            ipv6: profile.ipv6.clone(),
            on_apply: Rc::new(move |version, config| apply(version, config)),
        }),
    )
}
