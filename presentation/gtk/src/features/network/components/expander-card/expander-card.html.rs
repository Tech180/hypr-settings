use std::cell::Cell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use super::props::CardProps;
use crate::features::network::constants::SSID_MAX_CHARS;
use crate::shared::icon::{self, Icon, IconProps};

pub fn build(props: &CardProps) -> gtk::Widget {
    let mut classes = vec!["net-card"];
    if props.active {
        classes.push("net-card--active");
    }
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(classes)
        .build();

    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .transition_duration(220)
        .reveal_child(props.expanded)
        .child(&props.details)
        .build();
    revealer.add_css_class("net-card__details");

    let chevron = gtk::CenterBox::builder()
        .width_request(28)
        .height_request(28)
        .css_classes(["net-card__chevron"])
        .build();
    let set_chevron = {
        let chevron = chevron.clone();
        move |open: bool| {
            chevron.set_center_widget(Some(&icon::build(&IconProps {
                icon: if open {
                    Icon::ChevronUp
                } else {
                    Icon::ChevronDown
                },
                size: 18,
            })));
        }
    };
    set_chevron(props.expanded);

    let header = header(props, &chevron);
    header.set_focusable(true);
    header.set_accessible_role(gtk::AccessibleRole::Button);
    header.update_property(&[gtk::accessible::Property::Label(&format!(
        "{}, {}",
        props.title, props.subtitle
    ))]);
    header.set_cursor_from_name(Some("pointer"));

    let open = Rc::new(Cell::new(props.expanded));
    let toggle = {
        let (revealer, card, open, on_toggle) = (
            revealer.clone(),
            card.clone(),
            open.clone(),
            props.on_toggle.clone(),
        );
        move || {
            let now = !open.get();
            open.set(now);
            revealer.set_reveal_child(now);
            if now {
                card.add_css_class("is-expanded");
            } else {
                card.remove_css_class("is-expanded");
            }
            set_chevron(now);
            on_toggle(now);
        }
    };
    if props.expanded {
        card.add_css_class("is-expanded");
    }
    let click = gtk::GestureClick::new();
    click.connect_released({
        let toggle = toggle.clone();
        move |_, _, _, _| toggle()
    });
    header.add_controller(click);
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, key, _, _| {
        if matches!(
            key,
            gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter | gtk::gdk::Key::space
        ) {
            toggle();
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    header.add_controller(keys);

    card.append(&header);
    card.append(&revealer);
    card.upcast()
}

fn header(props: &CardProps, chevron: &gtk::CenterBox) -> gtk::Box {
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
        icon: props.icon,
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
            .label(&props.title)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(SSID_MAX_CHARS)
            .css_classes(["net-card__title"])
            .build(),
    );
    let sub = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .build();
    sub.append(
        &gtk::Label::builder()
            .label(&props.subtitle)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["net-card__subtitle"])
            .build(),
    );
    text.append(&sub);
    header.append(&text);

    let meta = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .valign(gtk::Align::Center)
        .css_classes(["net-card__meta"])
        .build();
    for widget in &props.meta {
        meta.append(widget);
    }
    if props.locked {
        let lock = icon::build(&IconProps {
            icon: Icon::Lock,
            size: 14,
        });
        lock.set_valign(gtk::Align::Center);
        lock.update_property(&[gtk::accessible::Property::Label("Secured network")]);
        meta.append(&lock);
    }
    meta.append(chevron);
    header.append(&meta);
    header
}
