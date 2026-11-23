use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use crate::shared::button::{self, ButtonKind, ButtonProps, ButtonSize};
use crate::shared::icon::{self, Icon, IconProps};
use crate::shared::section_header;
use crate::shared::section_header::{HeadingSize, SectionHeaderProps};

pub struct NavRowProps {
    pub icon: Icon,
    pub title: String,
    pub subtitle: String,
    /// Colours the subtitle, for the thing that is on.
    pub subtitle_accent: bool,
    /// A control before the chevron that does not open the page, like the Wi-Fi switch.
    pub trailing: Option<gtk::Widget>,
    pub on_open: Rc<dyn Fn()>,
}

/// A whole row that opens a page.
pub fn nav_row(props: &NavRowProps) -> gtk::Widget {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .focusable(true)
        .accessible_role(gtk::AccessibleRole::Button)
        .css_classes(["boxed-row", "boxed-row--clickable", "net-nav-row"])
        .build();
    row.update_property(&[gtk::accessible::Property::Label(&format!(
        "{}, {}",
        props.title, props.subtitle
    ))]);
    row.set_cursor_from_name(Some("pointer"));
    let lead = icon::build(&IconProps {
        icon: props.icon,
        size: 24,
    });
    lead.add_css_class("boxed-row__glyph");
    row.append(&lead);

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
            .css_classes(["boxed-row__title"])
            .build(),
    );
    let mut sub = vec!["boxed-row__subtitle"];
    if props.subtitle_accent {
        sub.push("net-accent-text");
    }
    text.append(
        &gtk::Label::builder()
            .label(&props.subtitle)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(sub)
            .build(),
    );
    row.append(&text);
    if let Some(trailing) = &props.trailing {
        row.append(trailing);
    }
    let chevron = icon::build(&IconProps {
        icon: Icon::ChevronRight,
        size: 18,
    });
    chevron.add_css_class("boxed-row__chevron");
    row.append(&chevron);

    let open = props.on_open.clone();
    let click = gtk::GestureClick::new();
    let trailing = props.trailing.clone();
    let owner = row.clone();
    click.connect_released(move |_, _, x, y| {
        // A click on the control before the chevron belongs to the control.
        let on_control = trailing.as_ref().is_some_and(|control| {
            owner
                .pick(x, y, gtk::PickFlags::DEFAULT)
                .is_some_and(|hit| hit == *control || hit.is_ancestor(control))
        });
        if !on_control {
            open();
        }
    });
    row.add_controller(click);
    let open = props.on_open.clone();
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, key, _, _| {
        if matches!(
            key,
            gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter | gtk::gdk::Key::space
        ) {
            open();
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    row.add_controller(keys);
    row.upcast()
}

/// A titled block: the heading (with optional controls at its right) over the body.
pub fn section(title: &str, trailing: Option<&gtk::Widget>, body: &gtk::Widget) -> gtk::Widget {
    section_spaced(title, trailing, body, 16)
}

pub fn section_spaced(
    title: &str,
    trailing: Option<&gtk::Widget>,
    body: &gtk::Widget,
    inner_gap: i32,
) -> gtk::Widget {
    let block = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(inner_gap)
        .build();
    let head = section_header::build(&SectionHeaderProps {
        title: title.to_owned(),
        subtitle: None,
        badge: None,
        tag: None,
        size: HeadingSize::Small,
    });
    if let Some(trailing) = trailing {
        let line = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .valign(gtk::Align::Center)
            .build();
        head.set_hexpand(true);
        line.append(&head);
        line.append(trailing);
        block.append(&line);
    } else {
        block.append(&head);
    }
    block.append(body);
    block.upcast()
}

pub fn card_list(cards: &[gtk::Widget]) -> gtk::Box {
    let list = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    for card in cards {
        list.append(card);
    }
    list
}

pub fn hint(text: &str) -> gtk::Widget {
    gtk::Label::builder()
        .label(text)
        .wrap(true)
        .max_width_chars(56)
        .css_classes(["net-hint"])
        .build()
        .upcast()
}

/// A centred glyph, a heading and a sentence, for pages with nothing to show.
pub fn empty_state(icon: Icon, title: &str, text: &str) -> gtk::Widget {
    let state = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(10)
        .halign(gtk::Align::Center)
        .css_classes(["net-empty"])
        .build();
    state.append(&icon::build(&IconProps { icon, size: 48 }));
    state.append(
        &gtk::Label::builder()
            .label(title)
            .css_classes(["net-empty__title"])
            .build(),
    );
    state.append(
        &gtk::Label::builder()
            .label(text)
            .wrap(true)
            .justify(gtk::Justification::Center)
            .max_width_chars(44)
            .css_classes(["net-empty__text"])
            .build(),
    );
    state.upcast()
}

/// A button for card details and toolbars.
pub fn action(
    label: &str,
    kind: ButtonKind,
    icon: Option<Icon>,
    on_click: Rc<dyn Fn()>,
) -> gtk::Widget {
    let mut props = ButtonProps::new(label, kind, on_click).size(ButtonSize::Compact);
    props.icon = icon;
    button::build(&props).upcast()
}

/// Icon-only control; `label` is the tooltip and the accessible name.
pub fn icon_action(
    label: &str,
    icon: Icon,
    kind: ButtonKind,
    on_click: Rc<dyn Fn()>,
) -> gtk::Widget {
    let mut classes = vec!["action-btn", "action-btn--icon-only"];
    classes.push(match kind {
        ButtonKind::Ghost => "action-btn--ghost",
        ButtonKind::Hollow => "action-btn--hollow",
        ButtonKind::Primary => "action-btn--primary",
        ButtonKind::Danger => "action-btn--danger",
    });
    classes.push("action-btn--compact");
    let glyph = icon::build(&IconProps {
        icon,
        size: 18,
    });
    glyph.add_css_class("action-btn__glyph");
    let button = gtk::Button::builder()
        .child(&glyph)
        .valign(gtk::Align::Center)
        .tooltip_text(label)
        .css_classes(classes)
        .build();
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    button.connect_clicked(move |_| on_click());
    button.upcast()
}
