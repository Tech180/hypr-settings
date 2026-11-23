use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;
use settings_domain::network::{IpConfig, IpMethod, IpVersion, validate_ip_config};

use super::props::IpSettingsProps;
use crate::features::network::components::common;
use crate::features::network::components::dialogs::frame::{self, entry, labelled, show_error};
use crate::shared::button::ButtonKind;
use crate::shared::segmented::{self, SegmentedProps};
use crate::shared::toggle::{self, ToggleProps};

pub fn build(props: &IpSettingsProps) -> gtk::Widget {
    let group = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["boxed-group", "ip-settings"])
        .overflow(gtk::Overflow::Hidden)
        .build();
    group.append(&version_stack(
        "IPv4",
        IpVersion::V4,
        &props.ipv4,
        props.on_apply.clone(),
    ));
    group.append(&version_stack(
        "IPv6",
        IpVersion::V6,
        &props.ipv6,
        props.on_apply.clone(),
    ));
    group.upcast()
}

fn version_stack(
    title: &str,
    version: IpVersion,
    config: &IpConfig,
    on_apply: Rc<dyn Fn(IpVersion, IpConfig)>,
) -> gtk::Widget {
    let enabled = config.method != IpMethod::Disabled;
    let manual = config.method == IpMethod::Manual;
    let fields = build_fields(config, version);

    let stack = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["ip-settings__stack"])
        .build();

    let head = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .css_classes(["boxed-row", "ip-settings__head"])
        .build();
    head.append(
        &gtk::Label::builder()
            .label(title)
            .xalign(0.0)
            .hexpand(true)
            .css_classes(["boxed-row__title"])
            .build(),
    );
    let apply = on_apply.clone();
    let fields_toggle = fields.clone();
    let switch = toggle::build(&ToggleProps {
        active: enabled,
        on_toggle: Rc::new(move |on| {
            let mut next = read_fields(&fields_toggle);
            next.method = if on {
                IpMethod::Auto
            } else {
                IpMethod::Disabled
            };
            apply(version, next);
        }),
    });
    head.append(&switch);
    stack.append(&head);
    let config_body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .css_classes(["ip-settings__body"])
        .build();
    let method_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .halign(gtk::Align::Start)
        .build();
    let manual_revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .transition_duration(220)
        .reveal_child(manual)
        .child(&manual_panel(&fields, version, on_apply.clone()))
        .build();
    manual_revealer.add_css_class("ip-settings__manual");

    let apply_method = on_apply.clone();
    let fields_ref = fields.clone();
    method_row.append(&segmented::build(&SegmentedProps {
        labels: vec!["Automatic".into(), "Manual".into()],
        selected: Some(usize::from(manual)),
        on_select: {
            let manual_revealer = manual_revealer.clone();
            Rc::new(move |index| {
                let manual = index == 1;
                manual_revealer.set_reveal_child(manual);
                if !manual {
                    let mut next = read_fields(&fields_ref);
                    next.method = IpMethod::Auto;
                    apply_method(version, next);
                }
            })
        },
    }));
    config_body.append(&method_row);
    config_body.append(&manual_revealer);

    let config_revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .transition_duration(220)
        .reveal_child(enabled)
        .child(&config_body)
        .build();
    stack.append(&config_revealer);
    stack.upcast()
}

#[derive(Clone)]
struct Fields {
    address: gtk::Entry,
    prefix: gtk::Entry,
    gateway: gtk::Entry,
    dns: gtk::Entry,
}

fn build_fields(config: &IpConfig, version: IpVersion) -> Fields {
    let fields = Fields {
        address: entry(&config.address, ""),
        prefix: entry(&config.prefix_or_mask, ""),
        gateway: entry(&config.gateway, ""),
        dns: entry(&config.dns.join(", "), ""),
    };
    let (address, prefix) = match version {
        IpVersion::V4 => ("192.168.1.10", "24 or 255.255.255.0"),
        IpVersion::V6 => ("2001:db8::10", "64"),
    };
    fields.address.set_placeholder_text(Some(address));
    fields.prefix.set_placeholder_text(Some(prefix));
    fields
}

fn read_fields(fields: &Fields) -> IpConfig {
    IpConfig {
        method: IpMethod::Manual,
        address: fields.address.text().trim().to_owned(),
        prefix_or_mask: fields.prefix.text().trim().to_owned(),
        gateway: fields.gateway.text().trim().to_owned(),
        dns: fields
            .dns
            .text()
            .split([',', ' '])
            .map(str::trim)
            .filter(|server| !server.is_empty())
            .map(str::to_owned)
            .collect(),
    }
}

fn manual_panel(
    fields: &Fields,
    version: IpVersion,
    on_apply: Rc<dyn Fn(IpVersion, IpConfig)>,
) -> gtk::Widget {
    let panel = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    let top = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();
    top.append(&labelled("Address", &fields.address));
    top.append(&labelled("Prefix / netmask", &fields.prefix));
    panel.append(&top);
    panel.append(&labelled("Gateway", &fields.gateway));
    panel.append(&labelled("DNS", &fields.dns));
    let error = frame::error_label();
    panel.append(&error);
    panel.append(&common::action(
        "Save",
        ButtonKind::Primary,
        None,
        {
            let fields = fields.clone();
            Rc::new(move || {
                let config = IpConfig {
                    method: IpMethod::Manual,
                    ..read_fields(&fields)
                };
                match validate_ip_config(version, &config) {
                    Ok(()) => {
                        error.set_visible(false);
                        on_apply(version, config);
                    }
                    Err(problem) => show_error(&error, &problem.message),
                }
            })
        },
    ));
    panel.upcast()
}
