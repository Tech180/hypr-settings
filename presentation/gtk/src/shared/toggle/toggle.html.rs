use gtk::prelude::*;
use gtk4 as gtk;

use super::props::ToggleProps;

pub fn build(props: &ToggleProps) -> gtk::Widget {
    let switch = gtk::Switch::builder()
        .active(props.active)
        .valign(gtk::Align::Center)
        .css_classes(["toggle-switch"])
        .build();
    let on_toggle = props.on_toggle.clone();
    switch.connect_active_notify(move |switch| on_toggle(switch.is_active()));
    switch.upcast()
}
