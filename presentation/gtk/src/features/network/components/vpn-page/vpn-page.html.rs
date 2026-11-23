use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;
use settings_domain::network::VpnProfile;

use crate::features::network::components::common;
use crate::features::network::constants::PAGE_GAP;
use crate::features::network::interfaces::VpnProps;
use crate::features::network::utils::{vpn_connectivity_badge, vpn_row_subtitle};
use crate::shared::badge::{self, BadgeProps};
use crate::shared::button::ButtonKind;
use crate::shared::icon::{self, Icon, IconProps};
use crate::shared::subpage_bar;

pub fn build(props: &VpnProps) -> gtk::Widget {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "page-content--sub", "network-page"])
        .build();
    let (active, other): (Vec<&VpnProfile>, Vec<&VpnProfile>) =
        props.view.profiles.iter().partition(|p| p.active);
    if props.view.profiles.is_empty() {
        column.append(&common::empty_state(
            Icon::Shield,
            "No VPN profiles",
            "Import a WireGuard or OpenVPN config, or create a WireGuard tunnel.",
        ));
    }
    if !active.is_empty() {
        column.append(&list("Active", &active, props));
    }
    if !other.is_empty() {
        column.append(&list("Available", &other, props));
    }

    let end = bar_actions(props);
    subpage_bar::frame_with_end(
        "VPN",
        props.actions.back.clone(),
        Some(&end),
        column.upcast_ref(),
    )
}

fn bar_actions(props: &VpnProps) -> gtk::Widget {
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    let import = props.actions.import_vpn.clone();
    bar.append(&common::icon_action(
        "Import VPN config",
        Icon::Upload,
        ButtonKind::Hollow,
        Rc::new(move || import()),
    ));
    let new = props.actions.new_vpn.clone();
    bar.append(&common::icon_action(
        "New WireGuard tunnel",
        Icon::Plus,
        ButtonKind::Primary,
        Rc::new(move || new()),
    ));
    bar.upcast()
}

fn list(title: &str, profiles: &[&VpnProfile], props: &VpnProps) -> gtk::Widget {
    let cards: Vec<gtk::Widget> = profiles.iter().map(|p| row(p, props)).collect();
    common::section(title, None, common::card_list(&cards).upcast_ref())
}

fn row(profile: &VpnProfile, props: &VpnProps) -> gtk::Widget {
    let actions = &props.actions;
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes([
            "net-card",
            if profile.active {
                "net-card--active"
            } else {
                ""
            },
            "net-card--nav",
        ])
        .build();

    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .css_classes(["net-card__header"])
        .build();

    let tile = gtk::CenterBox::builder()
        .width_request(40)
        .height_request(40)
        .css_classes(["net-card__icon"])
        .build();
    tile.set_center_widget(Some(&icon::build(&IconProps {
        icon: Icon::Shield,
        size: 20,
    })));
    header.append(&tile);

    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    text.append(
        &gtk::Label::builder()
            .label(&profile.name)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["net-card__title"])
            .build(),
    );
    text.append(
        &gtk::Label::builder()
            .label(vpn_row_subtitle(profile))
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["net-card__subtitle"])
            .build(),
    );
    header.append(&text);

    let meta = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .valign(gtk::Align::Center)
        .css_classes(["net-card__meta"])
        .build();
    if let Some((label, tone)) = vpn_connectivity_badge(profile) {
        meta.append(&badge::build(&BadgeProps::new(label, tone)));
    }
    let id = profile.id.clone();
    if profile.active {
        let off = actions.deactivate_vpn.clone();
        meta.append(&common::action(
            "Disconnect",
            ButtonKind::Hollow,
            None,
            Rc::new(move || off(id.clone())),
        ));
    } else {
        let on = actions.activate_vpn.clone();
        meta.append(&common::action(
            "Connect",
            ButtonKind::Primary,
            None,
            Rc::new(move || on(id.clone())),
        ));
    }
    let chevron = icon::build(&IconProps {
        icon: Icon::ChevronRight,
        size: 18,
    });
    chevron.add_css_class("net-card__chevron");
    meta.append(&chevron);
    header.append(&meta);

    wire_vpn_nav(
        &header,
        &meta,
        &profile.id,
        actions.open_vpn.clone(),
    );

    card.append(&header);
    card.upcast()
}

fn wire_vpn_nav(
    header: &gtk::Box,
    meta: &gtk::Box,
    profile_id: &str,
    on_open: Rc<dyn Fn(String)>,
) {
    header.set_focusable(true);
    header.set_accessible_role(gtk::AccessibleRole::Button);
    header.set_cursor_from_name(Some("pointer"));
    let header_owner = header.clone();
    let meta = meta.clone();
    let id = profile_id.to_owned();
    let click = gtk::GestureClick::new();
    click.connect_released({
        let on_open = on_open.clone();
        let id = id.clone();
        move |_, _, x, y| {
            if action_click(&header_owner, &meta, x, y) {
                return;
            }
            on_open(id.clone());
        }
    });
    header.add_controller(click);
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed({
        let on_open = on_open.clone();
        let id = id.clone();
        move |_, key, _, _| {
            if matches!(
                key,
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter | gtk::gdk::Key::space
            ) {
                on_open(id.clone());
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        }
    });
    header.add_controller(keys);
}

fn action_click(header: &gtk::Box, meta: &gtk::Box, x: f64, y: f64) -> bool {
    header.pick(x, y, gtk::PickFlags::DEFAULT).is_some_and(|hit| {
        meta.is_ancestor(&hit) && hit.has_css_class("action-btn")
    })
}
