use gtk::prelude::*;
use gtk4 as gtk;

use super::props::{BadgeProps, Tone};

pub fn build(props: &BadgeProps) -> gtk::Widget {
    gtk::Label::builder()
        .label(&props.text)
        .valign(gtk::Align::Center)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes([
            "hs-pill".to_owned(),
            format!("hs-pill--{}", props.tone.suffix()),
        ])
        .build()
        .upcast()
}

pub fn dot(tone: Tone) -> gtk::Widget {
    gtk::Label::builder()
        .valign(gtk::Align::Center)
        .css_classes(["hs-dot".to_owned(), format!("hs-dot--{}", tone.suffix())])
        .build()
        .upcast()
}
