use gtk::prelude::*;
use gtk4 as gtk;

use super::props::{ButtonKind, ButtonProps, ButtonSize};
use crate::shared::icon::{self, IconProps};

pub fn build(props: &ButtonProps) -> gtk::Button {
    let mut classes = vec!["action-btn"];
    classes.push(match props.kind {
        ButtonKind::Ghost => "action-btn--ghost",
        ButtonKind::Hollow => "action-btn--hollow",
        ButtonKind::Primary => "action-btn--primary",
        ButtonKind::Danger => "action-btn--danger",
    });
    match props.size {
        ButtonSize::Compact => classes.push("action-btn--compact"),
        ButtonSize::Regular => {}
        ButtonSize::Large => classes.push("action-btn--large"),
    }

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::Center)
        .build();
    if let Some(icon) = props.icon {
        let glyph = icon::build(&IconProps {
            icon,
            size: if props.size == ButtonSize::Compact {
                14
            } else {
                18
            },
        });
        glyph.add_css_class("action-btn__glyph");
        content.append(&glyph);
    }
    content.append(&gtk::Label::new(Some(&props.label)));

    let button = gtk::Button::builder()
        .child(&content)
        .valign(gtk::Align::Center)
        .sensitive(props.sensitive)
        .css_classes(classes)
        .build();
    if props.size == ButtonSize::Large {
        button.set_hexpand(true);
        button.set_halign(gtk::Align::Fill);
    }
    let on_click = props.on_click.clone();
    button.connect_clicked(move |_| on_click());
    button
}
