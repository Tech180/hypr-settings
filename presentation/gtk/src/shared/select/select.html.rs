use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use super::layout_constants::{DEFAULT_MIN_WIDTH, MAX_PANEL_HEIGHT};
use super::option_row::{option_row, set_trigger_label};
use super::props::SelectProps;
use crate::shared::icon::{self, Icon, IconProps};

pub fn build(props: &SelectProps) -> gtk::Widget {
    let selected = props.clamped_selected();
    let min_width = props.min_width.unwrap_or(DEFAULT_MIN_WIDTH);

    let root = gtk::Box::builder()
        .css_classes(["select"])
        .width_request(min_width)
        .build();

    let value_label = gtk::Label::builder()
        .hexpand(true)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    set_trigger_label(&value_label, props, selected);

    let icon_widget = icon::build(&IconProps {
        icon: Icon::ChevronsUpDown,
        size: 16,
    });
    icon_widget.add_css_class("select__icon");

    let trigger_body = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .hexpand(true)
        .build();
    trigger_body.append(&value_label);
    trigger_body.append(&icon_widget);

    let trigger = gtk::Button::builder()
        .child(&trigger_body)
        .css_classes(["select__trigger"])
        .hexpand(true)
        .accessible_role(gtk::AccessibleRole::ComboBox)
        .build();
    trigger.set_sensitive(!props.options.is_empty());
    root.append(&trigger);

    let options_list = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .css_classes(["select__options"])
        .build();

    let scroll = gtk::ScrolledWindow::builder()
        .max_content_height(MAX_PANEL_HEIGHT)
        .propagate_natural_height(true)
        .propagate_natural_width(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&options_list)
        .css_classes(["select__scroll"])
        .build();

    let popover = gtk::Popover::builder()
        .autohide(true)
        .has_arrow(false)
        .position(gtk::PositionType::Bottom)
        .child(&scroll)
        .css_classes(["select__popover"])
        .build();
    popover.set_parent(&trigger);
    // Sit on the trigger's bottom edge so the menu reads as attached.
    popover.set_offset(0, -1);

    if let Some(header) = &props.header_label {
        options_list.append(
            &gtk::Label::builder()
                .label(header)
                .xalign(0.0)
                .css_classes(["select__header"])
                .build(),
        );
    }

    let close: Rc<dyn Fn()> = {
        let popover = popover.clone();
        Rc::new(move || popover.popdown())
    };
    for (index, option) in props.options.iter().enumerate() {
        options_list.append(&option_row(
            option,
            index == selected,
            index,
            props.on_select.clone(),
            value_label.clone(),
            close.clone(),
        ));
    }

    trigger.connect_clicked({
        let popover = popover.clone();
        let root = root.clone();
        let scroll = scroll.clone();
        move |trigger| {
            if popover.is_visible() {
                popover.popdown();
                return;
            }
            scroll.set_min_content_width(trigger.width().max(min_width));
            root.add_css_class("select--open");
            popover.popup();
        }
    });
    popover.connect_closed({
        let root = root.clone();
        move |_| {
            root.remove_css_class("select--open");
        }
    });
    trigger.connect_destroy({
        let popover = popover.clone();
        move |_| popover.unparent()
    });

    root.upcast()
}
