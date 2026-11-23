use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use super::props::{FeatureGridProps, FeatureTileProps};
use crate::shared::icon::{self, IconProps};

pub fn build(props: &FeatureGridProps) -> gtk::Widget {
    let grid = gtk::Grid::builder()
        .column_homogeneous(true)
        .css_classes(["feature-grid"])
        .build();
    grid.set_column_spacing(12);
    grid.set_row_spacing(12);

    for (index, tile) in props.tiles.iter().enumerate() {
        let col = (index % 3) as i32;
        let row = (index / 3) as i32;
        grid.attach(&build_tile(tile), col, row, 1, 1);
    }
    grid.upcast()
}

fn build_tile(props: &FeatureTileProps) -> gtk::Widget {
    let supported = props.sensitive;
    let mut classes = vec!["feature-btn"];
    if props.active {
        classes.push("feature-btn--active");
    }
    if !supported {
        classes.push("feature-btn--unsupported");
    }
    let button = gtk::Button::builder().css_classes(classes).build();
    if let Some(tip) = &props.tooltip {
        button.set_tooltip_text(Some(tip));
    }

    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(10)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .build();
    let icon_host = gtk::CenterBox::builder()
        .css_classes(["feature-btn__icon"])
        .build();
    icon_host.set_center_widget(Some(&icon::build(&IconProps {
        icon: props.icon,
        size: 32,
    })));
    column.append(&icon_host);
    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .halign(gtk::Align::Center)
        .build();
    text.append(
        &gtk::Label::builder()
            .label(props.title)
            .css_classes(["feature-btn__title"])
            .halign(gtk::Align::Center)
            .build(),
    );
    text.append(
        &gtk::Label::builder()
            .label(props.subtitle)
            .css_classes(["feature-btn__subtitle"])
            .wrap(true)
            .justify(gtk::Justification::Center)
            .halign(gtk::Align::Center)
            .build(),
    );
    column.append(&text);
    button.set_child(Some(&column));
    button.set_halign(gtk::Align::Fill);
    button.set_hexpand(true);

    let active = props.active;
    let on_toggle = props.on_toggle.clone();
    let title = props.title;
    let reason = props
        .tooltip
        .clone()
        .unwrap_or_else(|| "This feature is not available on this display.".into());
    button.connect_clicked(move |btn| {
        if !supported {
            present_unavailable(btn, title, &reason);
            return;
        }
        on_toggle(!active);
    });
    button.upcast()
}

fn present_unavailable(anchor: &gtk::Button, title: &str, body: &str) {
    let dialog = adw::AlertDialog::builder()
        .heading(format!("{title} unavailable"))
        .body(body)
        .close_response("ok")
        .default_response("ok")
        .build();
    dialog.add_responses(&[("ok", "OK")]);
    dialog.connect_response(None, |_, _| {});
    dialog.present(Some(anchor));
}
