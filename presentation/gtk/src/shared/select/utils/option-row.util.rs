use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use crate::shared::icon::{self, Icon, IconProps};

use crate::shared::select::SelectProps;

pub fn set_trigger_label(label: &gtk::Label, props: &SelectProps, selected: usize) {
    match props.options.get(selected) {
        Some(text) => {
            label.set_label(text);
            label.set_css_classes(&["select__value"]);
        }
        None => {
            label.set_label(props.placeholder.as_deref().unwrap_or("Select…"));
            label.set_css_classes(&["select__value", "select__value--placeholder"]);
        }
    }
}

pub fn option_row(
    text: &str,
    is_selected: bool,
    index: usize,
    on_select: Rc<dyn Fn(usize)>,
    value_label: gtk::Label,
    close: Rc<dyn Fn()>,
) -> gtk::Button {
    let item = gtk::Button::builder()
        .hexpand(true)
        .can_focus(true)
        .accessible_role(gtk::AccessibleRole::Option)
        .css_classes(item_classes(is_selected))
        .build();
    item.update_property(&[gtk::accessible::Property::Label(text)]);

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .hexpand(true)
        .build();
    row.append(
        &gtk::Label::builder()
            .label(text)
            .hexpand(true)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );
    let tick = icon::build(&IconProps {
        icon: Icon::Check,
        size: 16,
    });
    tick.add_css_class("select__tick");
    row.append(&tick);
    item.set_child(Some(&row));

    let picked = text.to_owned();
    item.connect_clicked({
        let item = item.clone();
        move |_| {
            on_select(index);
            value_label.set_label(&picked);
            value_label.set_css_classes(&["select__value"]);
            mark_selected(&item);
            close();
        }
    });

    item
}

fn mark_selected(item: &gtk::Button) {
    if let Some(parent) = item.parent() {
        let mut child = parent.first_child();
        while let Some(current) = child {
            current.remove_css_class("select__item--selected");
            child = current.next_sibling();
        }
    }
    item.add_css_class("select__item--selected");
}

fn item_classes(selected: bool) -> Vec<String> {
    let mut classes = vec!["select__item".to_owned()];
    if selected {
        classes.push("select__item--selected".to_owned());
    }
    classes
}
