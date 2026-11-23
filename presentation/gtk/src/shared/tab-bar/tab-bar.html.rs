use gtk::prelude::*;
use gtk4 as gtk;

use super::props::TabBarProps;
use crate::shared::icon::{self, IconProps};

pub fn build(props: &TabBarProps) -> gtk::Widget {
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .css_classes(["tab-bar"])
        .build();
    for (index, tab) in props.tabs.iter().enumerate() {
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();
        if let Some(icon) = tab.icon {
            content.append(&icon::build(&IconProps { icon, size: 16 }));
        }
        content.append(&gtk::Label::new(Some(&tab.label)));
        let mut classes = vec!["tab-btn"];
        if index == props.selected {
            classes.push("tab-btn--active");
        }
        let button = gtk::Button::builder()
            .child(&content)
            .css_classes(classes)
            .build();
        let on_select = props.on_select.clone();
        button.connect_clicked(move |_| on_select(index));
        bar.append(&button);
    }
    bar.upcast()
}
