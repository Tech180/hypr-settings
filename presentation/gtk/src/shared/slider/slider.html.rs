use gtk::prelude::*;
use gtk4 as gtk;

use super::props::SliderProps;

fn format_value(value: f64, unit: &str) -> String {
    format!("{}{unit}", value.round())
}

pub fn build(props: &SliderProps) -> gtk::Widget {
    let scale = gtk::Scale::with_range(
        gtk::Orientation::Horizontal,
        props.min,
        props.max,
        props.step,
    );
    scale.set_value(props.value);
    scale.set_draw_value(false);
    scale.set_size_request(120, 28);
    scale.set_hexpand(true);
    scale.set_halign(gtk::Align::Fill);
    scale.set_valign(gtk::Align::Center);
    scale.add_css_class("hs-slider");

    let readout = gtk::Label::builder()
        .label(format_value(props.value, props.unit))
        .width_chars(7)
        .xalign(1.0)
        .css_classes(["slider-readout"])
        .build();
    let (unit, on_change) = (props.unit, props.on_change.clone());
    scale.connect_value_changed({
        let readout = readout.clone();
        move |scale| {
            readout.set_label(&format_value(scale.value(), unit));
            on_change(scale.value());
        }
    });

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    row.append(&scale);
    row.append(&readout);
    row.upcast()
}
