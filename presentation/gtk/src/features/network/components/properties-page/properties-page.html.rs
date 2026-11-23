use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;
use settings_domain::network::{ConnectionProfile, NetworkDevice};

use crate::features::network::components::common;
use crate::features::network::components::ip_settings::{self, IpSettingsProps};
use crate::features::network::components::telemetry_grid::{self, TelemetryItem};
use crate::features::network::constants::{PROPERTIES_PAGE_GAP, PROPERTIES_SECTION_GAP};
use crate::features::network::interfaces::PropertiesProps;
use crate::features::network::utils::{band_text, dns_text, format_bytes, or_dash};
use crate::shared::button::ButtonKind;
use crate::shared::icon::Icon;
use crate::shared::settings_group::{self, RowControl, SettingRowProps, SettingsGroupProps};
use crate::shared::subpage_bar;

pub fn build(props: &PropertiesProps) -> gtk::Widget {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PROPERTIES_PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes([
            "page-content",
            "page-content--sub",
            "network-page",
            "network-page--properties",
        ])
        .build();
    if let Some(profile) = &props.view.profile {
        column.append(&heading(profile, props));
        column.append(&flags(profile, props));
        column.append(&addressing(profile, props));
        column.append(&properties_section(
            "Telemetry",
            None,
            &telemetry(profile, props.view.device.as_ref()),
        ));
        column.append(&danger_zone(profile, props));
    } else {
        column.append(&common::empty_state(
            Icon::WifiOff,
            "Not connected to Wi-Fi",
            "Join a network to see its properties.",
        ));
    }
    subpage_bar::frame(
        "Connection Properties",
        props.actions.back.clone(),
        column.upcast_ref(),
    )
}

fn properties_section(
    title: &str,
    trailing: Option<&gtk::Widget>,
    body: &gtk::Widget,
) -> gtk::Widget {
    common::section_spaced(title, trailing, body, PROPERTIES_SECTION_GAP)
}

fn heading(profile: &ConnectionProfile, props: &PropertiesProps) -> gtk::Widget {
    let title = if profile.ssid.is_empty() {
        profile.name.clone()
    } else {
        profile.ssid.clone()
    };
    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    text.append(
        &gtk::Label::builder()
            .label(&title)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .tooltip_text(&title)
            .css_classes(["section-title"])
            .build(),
    );
    text.append(
        &gtk::Label::builder()
            .label(or_dash(&profile.security))
            .xalign(0.0)
            .css_classes(["section-subtitle"])
            .build(),
    );

    let meta = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .valign(gtk::Align::Center)
        .build();
    if profile.frequency_mhz > 0 {
        meta.append(
            &gtk::Label::builder()
                .label(format!("{} MHz", profile.frequency_mhz))
                .valign(gtk::Align::Center)
                .css_classes(["hs-badge"])
                .build(),
        );
    }
    let share = props.actions.share.clone();
    let id = profile.id.clone();
    meta.append(&common::icon_action(
        "Share Wi-Fi QR code",
        Icon::QrCode,
        ButtonKind::Hollow,
        Rc::new(move || share(id.clone())),
    ));

    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .css_classes(["section-header"])
        .build();
    header.append(&text);
    header.append(&meta);
    header.upcast()
}

fn switch_row(
    title: &str,
    subtitle: &str,
    active: bool,
    id: &str,
    set: &Rc<dyn Fn(String, bool)>,
) -> SettingRowProps {
    let (set, id) = (set.clone(), id.to_owned());
    SettingRowProps {
        title: title.into(),
        subtitle: Some(subtitle.into()),
        sensitive: true,
        control: RowControl::Switch {
            active,
            on_toggle: Rc::new(move |on| set(id.clone(), on)),
        },
        leading: None,
        badge: None,
    }
}

fn flags(profile: &ConnectionProfile, props: &PropertiesProps) -> gtk::Widget {
    let actions = &props.actions;
    let group = settings_group::build(&SettingsGroupProps {
        rows: vec![
            switch_row(
                "Connect automatically",
                "Join this network whenever it is in range",
                profile.autoconnect,
                &profile.id,
                &actions.set_autoconnect,
            ),
            switch_row(
                "Metered connection",
                "Limit background data on this network",
                profile.metered,
                &profile.id,
                &actions.set_metered,
            ),
            switch_row(
                "Randomize MAC address",
                "Use a different hardware address for this network",
                profile.mac_randomization.is_random(),
                &profile.id,
                &actions.set_mac_random,
            ),
        ],
    });
    properties_section("Options", None, &group)
}

fn addressing(profile: &ConnectionProfile, props: &PropertiesProps) -> gtk::Widget {
    let apply = props.apply_ip.clone();
    properties_section(
        "IP settings",
        None,
        &ip_settings::build(&IpSettingsProps {
            ipv4: profile.ipv4.clone(),
            ipv6: profile.ipv6.clone(),
            on_apply: Rc::new(move |version, config| apply(version, config)),
        }),
    )
}

fn telemetry(profile: &ConnectionProfile, device: Option<&NetworkDevice>) -> gtk::Widget {
    let mut items = vec![TelemetryItem::new("Standard", or_dash(&profile.standard))];
    if profile.frequency_mhz > 0 {
        items.push(TelemetryItem::new("Band", band_text(profile.frequency_mhz)));
    }
    items.extend([
        TelemetryItem::new("Signal", format!("{}%", profile.strength)),
        TelemetryItem::new("BSSID", or_dash(&profile.bssid)).mono(),
    ]);
    if let Some(device) = device {
        let bytes = |value: Option<u64>| value.map_or_else(|| "—".to_owned(), format_bytes);
        items.extend([
            TelemetryItem::new("Interface", or_dash(&device.iface)),
            TelemetryItem::new("IP address", or_dash(&device.ip)).mono(),
            TelemetryItem::new("Gateway", or_dash(&device.gateway)).mono(),
            TelemetryItem::new("DNS", dns_text(&device.dns)).mono(),
            TelemetryItem::new("IPv6", or_dash(&device.ipv6)).mono(),
            TelemetryItem::new("MAC", or_dash(&device.mac)).mono(),
            TelemetryItem::new(
                "Link rate",
                device
                    .speed_mbps
                    .map_or_else(|| "—".to_owned(), |mbps| format!("{mbps} Mbps")),
            ),
            TelemetryItem::new(
                "MTU",
                device
                    .mtu
                    .map_or_else(|| "—".to_owned(), |mtu| mtu.to_string()),
            ),
            TelemetryItem::new("Received", bytes(device.rx_bytes)),
            TelemetryItem::new("Sent", bytes(device.tx_bytes)),
        ]);
    }
    let panel = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["telemetry-panel"])
        .build();
    panel.append(&telemetry_grid::build_spaced(&items, 10, 14));
    panel.upcast()
}

fn danger_zone(profile: &ConnectionProfile, props: &PropertiesProps) -> gtk::Widget {
    let forget = props.actions.forget.clone();
    let id = profile.id.clone();
    common::action(
        "Forget network",
        ButtonKind::Danger,
        Some(Icon::Trash),
        Rc::new(move || forget(id.clone())),
    )
}
