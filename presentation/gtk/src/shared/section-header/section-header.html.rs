use gtk::prelude::*;
use gtk4 as gtk;

use super::props::{HeadingSize, SectionHeaderProps};

pub fn build(props: &SectionHeaderProps) -> gtk::Box {
    let title_class = match props.size {
        HeadingSize::Large => "section-title",
        HeadingSize::Small => "section-title--small",
    };
    let title_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();
    title_row.append(
        &gtk::Label::builder()
            .label(&props.title)
            .xalign(0.0)
            // Panel descriptions differ in length per display; they shrink with an
            // ellipsis rather than widening the page and the window.
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .hexpand(true)
            .tooltip_text(&props.title)
            .css_classes(["section-title", title_class])
            .build(),
    );
    if let Some(badge) = &props.badge {
        title_row.append(
            &gtk::Label::builder()
                .label(badge)
                .valign(gtk::Align::Center)
                .css_classes(["hs-badge"])
                .build(),
        );
    }

    if let Some(tag) = &props.tag {
        title_row.append(
            &gtk::Label::builder()
                .label(&tag.text)
                .valign(gtk::Align::Center)
                .tooltip_text(&tag.tooltip)
                .css_classes(["hs-badge", "hs-badge--tag"])
                .build(),
        );
    }

    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .valign(gtk::Align::End)
        .build();
    text.append(&title_row);
    if let Some(subtitle) = &props.subtitle {
        text.append(
            &gtk::Label::builder()
                .label(subtitle)
                .xalign(0.0)
                .wrap(true)
                .css_classes(["section-subtitle"])
                .build(),
        );
    }

    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .css_classes(["section-header"])
        .build();
    header.append(&text);
    header
}
