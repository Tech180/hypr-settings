use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;
use settings_domain::network::AccessPoint;

use crate::features::network::components::common;
use crate::features::network::components::expander_card::{self, CardProps};
use crate::features::network::components::password_field::{self, PasswordFieldProps};
use crate::features::network::components::telemetry_grid::{self, TelemetryItem};
use crate::features::network::constants::{PAGE_GAP, SSID_MAX_CHARS};
use crate::features::network::interfaces::{WifiProps, WifiSavedEntry};
use crate::features::network::utils::{
    ap_subtitle, available_networks, band_text, device_label, filter_access_points,
    filter_saved_connections, or_dash, scan_policy_from_index, scan_policy_index,
    scan_policy_labels, security_badge, wifi_icon,
};
use crate::shared::badge::{self, BadgeProps, Tone};
use crate::shared::button::ButtonKind;
use crate::shared::dropdown::{self, DropdownProps};
use crate::shared::icon::{self, Icon, IconProps};
use crate::shared::pagination::{
    PaginationProps, PAGE_SIZE_OPTIONS, build as build_pagination, slice,
};
use crate::shared::search_field::{self, SearchFieldProps};
use crate::shared::subpage_bar;
use crate::shared::toggle::{self, ToggleProps};

#[derive(Clone)]
pub struct WifiListSurfaces {
    pub saved_slot: gtk::Box,
    pub saved_pager: gtk::Box,
    pub results: gtk::Box,
    pub available_pager: gtk::Box,
}

pub fn build(props: &WifiProps) -> (gtk::Widget, WifiListSurfaces) {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "page-content--sub", "network-page"])
        .build();

    let available = available_networks(&props.view);
    let saved_slot = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    let saved_pager = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let results = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    let available_pager = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let surfaces = WifiListSurfaces {
        saved_slot: saved_slot.clone(),
        saved_pager: saved_pager.clone(),
        results: results.clone(),
        available_pager: available_pager.clone(),
    };

    column.append(&toolbar(props));
    if props.view.enabled {
        refresh_wifi_lists(
            &saved_slot,
            &saved_pager,
            &results,
            &available_pager,
            &available,
            props,
            &props.query,
        );
        column.append(&networks(
            props,
            &saved_slot,
            &saved_pager,
            &results,
            &available_pager,
        ));
    } else {
        column.append(&common::empty_state(
            Icon::WifiOff,
            "Wi-Fi is off",
            "Turn Wi-Fi on to see the networks around you.",
        ));
    }
    (
        subpage_bar::frame("Wi-Fi", props.actions.back.clone(), column.upcast_ref()),
        surfaces,
    )
}

fn toolbar(props: &WifiProps) -> gtk::Widget {
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .valign(gtk::Align::Center)
        .css_classes(["net-toolbar"])
        .build();

    let set = props.actions.set_wifi_enabled.clone();
    let switch = toggle::build(&ToggleProps {
        active: props.view.enabled,
        on_toggle: Rc::new(move |on| set(on)),
    });
    switch.set_valign(gtk::Align::Center);
    bar.append(&switch);

    if !props.view.devices.is_empty() {
        let labels: Vec<String> = props.view.devices.iter().map(device_label).collect();
        let ifaces: Vec<String> = props.view.devices.iter().map(|d| d.iface.clone()).collect();
        let selected = ifaces
            .iter()
            .position(|iface| *iface == props.view.selected)
            .unwrap_or(0);
        let select = props.actions.select_wifi_adapter.clone();
        bar.append(&dropdown::build(&DropdownProps {
            options: labels,
            selected,
            on_select: Rc::new(move |index| {
                if let Some(iface) = ifaces.get(index) {
                    select(iface.clone());
                }
            }),
        }));
    }

    if props.view.enabled {
        let on_query = props.actions.set_query.clone();
        let search = search_field::build(&SearchFieldProps {
            on_change: Rc::new(move |query| on_query(query)),
            ..SearchFieldProps::new("Search networks", Rc::new(|_| {})).text(&props.query)
        });
        search.set_hexpand(true);
        search.set_halign(gtk::Align::Fill);
        bar.append(&search);
    }

    bar.upcast()
}

fn networks(
    props: &WifiProps,
    saved_slot: &gtk::Box,
    saved_pager: &gtk::Box,
    results: &gtk::Box,
    available_pager: &gtk::Box,
) -> gtk::Widget {
    let block = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .build();
    if !props.view.saved.is_empty() {
        let saved_body = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .build();
        saved_body.append(saved_slot);
        saved_body.append(saved_pager);
        block.append(&common::section(
            "Saved Connections",
            None,
            saved_body.upcast_ref(),
        ));
    }

    let available_body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    available_body.append(results);
    available_body.append(available_pager);
    block.append(&common::section(
        "Available Networks",
        Some(&scan_controls(props)),
        available_body.upcast_ref(),
    ));
    block.upcast()
}

fn scan_controls(props: &WifiProps) -> gtk::Widget {
    let controls = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .valign(gtk::Align::Center)
        .build();
    if props.scanning {
        let spinner = gtk::Spinner::builder().valign(gtk::Align::Center).build();
        spinner.start();
        spinner.update_property(&[gtk::accessible::Property::Label("Scanning")]);
        controls.append(&spinner);
    }
    let set = props.actions.set_scan_policy.clone();
    let interval = props.prefs.interval_ms;
    controls.append(&dropdown::build(&DropdownProps {
        options: scan_policy_labels(),
        selected: scan_policy_index(&props.prefs),
        on_select: Rc::new(move |index| {
            let (policy, interval_ms) = scan_policy_from_index(index, interval);
            set(policy, interval_ms);
        }),
    }));
    controls.append(&common::icon_action(
        "Scan for networks",
        Icon::RefreshCw,
        ButtonKind::Hollow,
        props.actions.scan.clone(),
    ));
    controls.upcast()
}

pub fn refresh_wifi_lists(
    saved_slot: &gtk::Box,
    saved_pager: &gtk::Box,
    results_slot: &gtk::Box,
    available_pager: &gtk::Box,
    available: &[AccessPoint],
    props: &WifiProps,
    query: &str,
) {
    fill_saved(saved_slot, saved_pager, props, query);
    fill_results(results_slot, available_pager, available, props, query);
}

fn clear_box(slot: &gtk::Box) {
    while let Some(child) = slot.first_child() {
        slot.remove(&child);
    }
}

fn fill_saved(slot: &gtk::Box, pager: &gtk::Box, props: &WifiProps, query: &str) {
    clear_box(slot);
    clear_box(pager);
    let shown = filter_saved_connections(&props.view.saved, query);
    if shown.is_empty() {
        if !query.trim().is_empty() && !props.view.saved.is_empty() {
            slot.append(&common::hint("No saved connections match your search."));
        }
        return;
    }
    let page_size = props.pagination.saved_page_size;
    let pages = slice::total_pages(shown.len(), page_size);
    let page = slice::clamp_page(props.pagination.saved_page, pages);
    for entry in slice::slice_page(&shown, page, page_size) {
        slot.append(&saved_card(entry, props));
    }
    pager.append(&build_pagination(&saved_pagination_props(
        props, page, pages, page_size,
    )));
}

fn fill_results(
    slot: &gtk::Box,
    pager: &gtk::Box,
    all: &[AccessPoint],
    props: &WifiProps,
    query: &str,
) {
    clear_box(slot);
    clear_box(pager);
    let shown = filter_access_points(all, query);
    if shown.is_empty() {
        let text = if all.is_empty() {
            if props.scanning {
                "Looking for networks…"
            } else {
                "No networks found. Use Scan to look again."
            }
        } else {
            "No networks match your search."
        };
        slot.append(&common::hint(text));
        return;
    }
    let page_size = props.pagination.available_page_size;
    let pages = slice::total_pages(shown.len(), page_size);
    let page = slice::clamp_page(props.pagination.available_page, pages);
    for ap in slice::slice_page(&shown, page, page_size) {
        slot.append(&card(ap, props));
    }
    pager.append(&build_pagination(&available_pagination_props(
        props, page, pages, page_size,
    )));
}

fn saved_pagination_props(
    props: &WifiProps,
    page: usize,
    total_pages: usize,
    page_size: usize,
) -> PaginationProps {
    PaginationProps {
        page,
        total_pages,
        on_page: props.actions.set_wifi_saved_page.clone(),
        page_size,
        page_size_options: PAGE_SIZE_OPTIONS.to_vec(),
        on_page_size: props.actions.set_wifi_saved_page_size.clone(),
        accessible_label: Some("Saved connections pagination".into()),
    }
}

fn available_pagination_props(
    props: &WifiProps,
    page: usize,
    total_pages: usize,
    page_size: usize,
) -> PaginationProps {
    PaginationProps {
        page,
        total_pages,
        on_page: props.actions.set_wifi_available_page.clone(),
        page_size,
        page_size_options: PAGE_SIZE_OPTIONS.to_vec(),
        on_page_size: props.actions.set_wifi_available_page_size.clone(),
        accessible_label: Some("Available networks pagination".into()),
    }
}

fn find_ap<'a>(props: &'a WifiProps, ssid: &str) -> Option<&'a AccessPoint> {
    props
        .view
        .access_points
        .iter()
        .find(|ap| ap.ssid == ssid && ap.connected)
        .or_else(|| props.view.access_points.iter().find(|ap| ap.ssid == ssid))
}

fn saved_card(entry: &WifiSavedEntry, props: &WifiProps) -> gtk::Widget {
    let ap = find_ap(props, &entry.ssid);
    if entry.active {
        return active_saved_card(entry, ap, props);
    }
    let actions = &props.actions;
    let toggled = actions.card_toggled.clone();
    let key = entry.id.clone();
    let subtitle = saved_subtitle(entry, ap);
    let locked = ap.is_some_and(|ap| ap.requires_password);
    let icon = ap
        .map(|ap| wifi_icon(ap.signal_bars()))
        .unwrap_or(Icon::WifiHigh);
    expander_card::build(&CardProps {
        title: entry.ssid.clone(),
        subtitle,
        locked,
        icon,
        active: false,
        expanded: props.memory.expanded.contains(&entry.id),
        meta: Vec::new(),
        details: saved_details(entry, ap, props),
        on_toggle: Rc::new(move |open| toggled(key.clone(), !open)),
    })
}

fn active_saved_card(
    entry: &WifiSavedEntry,
    ap: Option<&AccessPoint>,
    props: &WifiProps,
) -> gtk::Widget {
    let actions = &props.actions;
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["net-card", "net-card--active", "net-card--nav"])
        .build();

    let icon = ap
        .map(|ap| wifi_icon(ap.signal_bars()))
        .unwrap_or(Icon::Wifi);
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
        icon,
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
            .label(&entry.ssid)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(SSID_MAX_CHARS)
            .css_classes(["net-card__title"])
            .build(),
    );
    text.append(
        &gtk::Label::builder()
            .label("Connected")
            .xalign(0.0)
            .css_classes(["net-card__subtitle", "net-accent-text"])
            .build(),
    );
    header.append(&text);

    let meta = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .valign(gtk::Align::Center)
        .css_classes(["net-card__meta"])
        .build();
    if let Some(ap) = ap {
        if ap.frequency_mhz > 0 {
            meta.append(&badge::build(&BadgeProps::new(
                band_text(ap.frequency_mhz),
                Tone::Neutral,
            )));
        }
        meta.append(&badge::build(&BadgeProps::new(
            security_badge(ap),
            Tone::Neutral,
        )));
    }
    let disconnect = common::action(
        "Disconnect",
        ButtonKind::Hollow,
        None,
        actions.disconnect_wifi.clone(),
    );
    meta.append(&disconnect);
    let chevron = icon::build(&IconProps {
        icon: Icon::ChevronRight,
        size: 18,
    });
    chevron.add_css_class("net-card__chevron");
    meta.append(&chevron);
    header.append(&meta);

    let open = actions.open_properties.clone();
    wire_properties_nav(&header, &disconnect, open);
    header.update_property(&[gtk::accessible::Property::Label(&format!(
        "{}, Connection properties",
        entry.ssid
    ))]);
    card.append(&header);
    card.upcast()
}

fn wire_properties_nav(header: &gtk::Box, disconnect: &gtk::Widget, on_open: Rc<dyn Fn()>) {
    header.set_focusable(true);
    header.set_accessible_role(gtk::AccessibleRole::Button);
    header.set_cursor_from_name(Some("pointer"));
    let header_owner = header.clone();
    let disconnect = disconnect.clone();
    let click = gtk::GestureClick::new();
    click.connect_released({
        let on_open = on_open.clone();
        move |_, _, x, y| {
            let on_disconnect = header_owner
                .pick(x, y, gtk::PickFlags::DEFAULT)
                .is_some_and(|hit| hit == disconnect || hit.is_ancestor(&disconnect));
            if !on_disconnect {
                on_open();
            }
        }
    });
    header.add_controller(click);
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed({
        let on_open = on_open.clone();
        move |_, key, _, _| {
            if matches!(
                key,
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter | gtk::gdk::Key::space
            ) {
                on_open();
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        }
    });
    header.add_controller(keys);
}

fn saved_subtitle(entry: &WifiSavedEntry, ap: Option<&AccessPoint>) -> String {
    if entry.active {
        ap.map(ap_subtitle)
            .unwrap_or_else(|| "Connected".to_owned())
    } else if let Some(ap) = ap {
        ap_subtitle(ap)
    } else {
        "Saved network".to_owned()
    }
}

fn saved_details(
    entry: &WifiSavedEntry,
    ap: Option<&AccessPoint>,
    props: &WifiProps,
) -> gtk::Widget {
    let actions = &props.actions;
    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .css_classes(["net-card__body"])
        .build();
    if let Some(ap) = ap {
        body.append(&ap_telemetry(ap));
    } else {
        body.append(&common::hint(
            "This network is not in range right now. Connect to try anyway.",
        ));
    }

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .css_classes(["details-actions"])
        .build();
    if entry.active {
        row.append(&common::icon_action(
            "Connection properties",
            Icon::Info,
            ButtonKind::Hollow,
            actions.open_properties.clone(),
        ));
    } else {
        let activate = actions.activate_wifi.clone();
        let id = entry.id.clone();
        row.append(&common::action(
            "Connect",
            ButtonKind::Primary,
            None,
            Rc::new(move || activate(id.clone())),
        ));
    }
    body.append(&row);
    body.upcast()
}

fn ap_telemetry(ap: &AccessPoint) -> gtk::Widget {
    let security = if ap.security.is_empty() || !ap.requires_password {
        "None (open)".to_owned()
    } else {
        ap.security.clone()
    };
    telemetry_grid::build(&[
        TelemetryItem::new("Security", security),
        TelemetryItem::new("Band", band_text(ap.frequency_mhz)),
        TelemetryItem::new(
            "Channel",
            format!("{} ({} MHz)", ap.channel, ap.frequency_mhz),
        ),
        TelemetryItem::new("Signal", format!("{}%", ap.strength)),
        TelemetryItem::new("BSSID", or_dash(&ap.bssid)).mono(),
        TelemetryItem::new(
            "Link rate",
            if ap.rate_mbps == 0 {
                "—".to_owned()
            } else {
                format!("{} Mbps", ap.rate_mbps)
            },
        ),
        TelemetryItem::new("Device", or_dash(&ap.device)),
    ])
}

fn card(ap: &AccessPoint, props: &WifiProps) -> gtk::Widget {
    let actions = &props.actions;
    let mut meta = Vec::new();
    if ap.connected {
        meta.push(common::action(
            "Disconnect",
            ButtonKind::Hollow,
            None,
            actions.disconnect_wifi.clone(),
        ));
    }
    let toggled = actions.card_toggled.clone();
    let key = ap.ssid.clone();
    expander_card::build(&CardProps {
        title: ap.ssid.clone(),
        subtitle: ap_subtitle(ap),
        locked: ap.requires_password,
        icon: wifi_icon(ap.signal_bars()),
        active: ap.connected,
        expanded: props.memory.expanded.contains(&ap.ssid),
        meta,
        details: details(ap, props),
        on_toggle: Rc::new(move |open| toggled(key.clone(), open)),
    })
}

fn details(ap: &AccessPoint, props: &WifiProps) -> gtk::Widget {
    let actions = &props.actions;
    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .css_classes(["net-card__body"])
        .build();
    body.append(&ap_telemetry(ap));

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .css_classes(["details-actions"])
        .build();
    if ap.connected {
        row.append(&common::icon_action(
            "Connection properties",
            Icon::Info,
            ButtonKind::Hollow,
            actions.open_properties.clone(),
        ));
    } else {
        row.append(&connect_controls(ap, props));
    }
    body.append(&row);
    body.upcast()
}

/// Open networks join with one click; secured ones take a password first. An empty
/// password joins with the credentials NetworkManager already has saved.
fn connect_controls(ap: &AccessPoint, props: &WifiProps) -> gtk::Widget {
    let actions = &props.actions;
    let connect = actions.connect_wifi.clone();
    let ssid = ap.ssid.clone();
    if !ap.requires_password {
        return common::action(
            "Connect",
            ButtonKind::Primary,
            None,
            Rc::new(move || connect(ssid.clone(), None)),
        );
    }
    let typed = Rc::new(std::cell::RefCell::new(
        props
            .memory
            .drafts
            .get(&ap.ssid)
            .cloned()
            .unwrap_or_default(),
    ));
    let submit: Rc<dyn Fn()> = {
        let (typed, ssid) = (typed.clone(), ssid.clone());
        Rc::new(move || {
            let password = typed.borrow().clone();
            connect(ssid.clone(), (!password.is_empty()).then_some(password));
        })
    };
    let draft = actions.set_draft.clone();
    let field = password_field::build(&PasswordFieldProps {
        placeholder: "Password".to_owned(),
        text: typed.borrow().clone(),
        on_change: {
            let (typed, ssid) = (typed.clone(), ssid);
            Rc::new(move |text| {
                draft(ssid.clone(), text.clone());
                *typed.borrow_mut() = text;
            })
        },
        on_submit: submit.clone(),
    });
    let line = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .hexpand(true)
        .build();
    line.append(&field);
    line.append(&common::action(
        "Connect",
        ButtonKind::Primary,
        None,
        submit,
    ));
    line.upcast()
}
