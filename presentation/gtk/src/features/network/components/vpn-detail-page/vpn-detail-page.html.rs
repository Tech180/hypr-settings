use std::cell::Cell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;
use settings_domain::network::VpnProfile;

use crate::features::network::components::common;
use crate::features::network::components::telemetry_grid::{self, TelemetryItem};
use crate::features::network::constants::PAGE_GAP;
use crate::features::network::interfaces::VpnDetailProps;
use crate::features::network::utils::{dns_text, or_dash, vpn_subtitle};
use crate::shared::button::ButtonKind;
use crate::shared::icon::{self, Icon, IconProps};
use crate::shared::subpage_bar;

pub fn build(props: &VpnDetailProps) -> gtk::Widget {
    let profile = &props.profile;
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "page-content--sub", "network-page"])
        .build();
    column.append(&heading(profile));
    column.append(&telemetry(profile));
    if !profile.peer_public_key.is_empty() {
        column.append(&common::section(
            "Peer",
            None,
            &peer_key(&profile.peer_public_key),
        ));
    }
    if profile.editable {
        column.append(&actions(profile, &props.actions));
    }
    subpage_bar::frame(&profile.name, props.actions.back.clone(), column.upcast_ref())
}

fn heading(profile: &VpnProfile) -> gtk::Widget {
    let block = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .css_classes(["section-header"])
        .build();
    block.append(
        &gtk::Label::builder()
            .label(&profile.name)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["section-title"])
            .build(),
    );
    block.append(
        &gtk::Label::builder()
            .label(vpn_subtitle(profile))
            .xalign(0.0)
            .css_classes(["section-subtitle", "net-accent-text"])
            .build(),
    );
    block.upcast()
}

fn telemetry(profile: &VpnProfile) -> gtk::Widget {
    let mut items = vec![
        TelemetryItem::new("Interface", or_dash(&profile.interface_name)),
        TelemetryItem::new("IPv4", or_dash(&profile.ipv4_address)).mono(),
        TelemetryItem::new("IPv6", or_dash(&profile.ipv6_address)).mono(),
        TelemetryItem::new("Gateway", or_dash(&profile.gateway)).mono(),
        TelemetryItem::new("DNS", dns_text(&profile.dns)).mono(),
    ];
    if let Some(mtu) = profile.mtu {
        items.push(TelemetryItem::new("MTU", mtu.to_string()));
    }
    if let Some(port) = profile.listen_port {
        items.push(TelemetryItem::new("Listen port", port.to_string()));
    }
    if profile.peer_count > 0 {
        items.push(TelemetryItem::new("Peers", profile.peer_count.to_string()));
    }
    items.push(TelemetryItem::new(
        "Autoconnect",
        if profile.autoconnect { "Yes" } else { "No" },
    ));
    if !profile.systemd_unit.is_empty() {
        items.push(TelemetryItem::new("Unit", profile.systemd_unit.clone()));
    }
    common::section(
        "Details",
        None,
        &telemetry_grid::build(&items),
    )
}

fn peer_key(key: &str) -> gtk::Widget {
    let panel = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .css_classes(["telemetry-panel"])
        .build();
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();
    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .build();
    text.append(
        &gtk::Label::builder()
            .label("PUBLIC KEY")
            .xalign(0.0)
            .css_classes(["detail-item__label"])
            .build(),
    );
    let masked = "••••••••••••••••••••••••••••••••••••••••••••••••••••••••••••••••";
    let value = gtk::Label::builder()
        .label(masked)
        .xalign(0.0)
        .selectable(true)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::Char)
        .css_classes(["detail-item__value", "detail-item__value--mono"])
        .build();
    text.append(&value);
    row.append(&text);

    let shown = Rc::new(Cell::new(false));
    let full = key.to_owned();
    let toggle = gtk::Button::builder()
        .valign(gtk::Align::Center)
        .tooltip_text("Show public key")
        .css_classes([
            "action-btn",
            "action-btn--ghost",
            "action-btn--icon-only",
            "action-btn--compact",
        ])
        .build();
    toggle.set_child(Some(&icon::build(&IconProps {
        icon: Icon::Eye,
        size: 18,
    })));
    toggle.connect_clicked({
        let (shown, value, toggle) = (shown.clone(), value.clone(), toggle.clone());
        move |_| {
            let now = !shown.get();
            shown.set(now);
            value.set_label(if now { &full } else { masked });
            toggle.set_child(Some(&icon::build(&IconProps {
                icon: if now {
                    Icon::EyeOff
                } else {
                    Icon::Eye
                },
                size: 18,
            })));
            toggle.update_property(&[gtk::accessible::Property::Label(if now {
                "Hide public key"
            } else {
                "Show public key"
            })]);
        }
    });
    row.append(&toggle);
    panel.append(&row);
    panel.upcast()
}

fn actions(profile: &VpnProfile, actions: &crate::features::network::interfaces::NetworkActions) -> gtk::Widget {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();
    let edit = actions.edit_vpn.clone();
    let id = profile.id.clone();
    row.append(&common::action(
        "Edit",
        ButtonKind::Hollow,
        Some(Icon::Pencil),
        Rc::new(move || edit(id.clone())),
    ));
    let delete = actions.delete_vpn.clone();
    let id = profile.id.clone();
    row.append(&common::action(
        "Delete",
        ButtonKind::Danger,
        Some(Icon::Trash),
        Rc::new(move || delete(id.clone())),
    ));
    common::section("Manage", None, row.upcast_ref())
}
