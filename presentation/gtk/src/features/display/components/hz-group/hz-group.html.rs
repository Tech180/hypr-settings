use gtk::prelude::*;
use gtk4 as gtk;

use super::props::HzGroupProps;

pub fn build(props: &HzGroupProps) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .css_classes(["hz-group"])
        .build();

    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .css_classes(["hz-header"])
        .build();
    let titles = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    titles.append(
        &gtk::Label::builder()
            .label("Refresh Rate")
            .xalign(0.0)
            .yalign(0.0)
            .valign(gtk::Align::Start)
            .css_classes(["hz-title"])
            .build(),
    );
    titles.append(
        &gtk::Label::builder()
            .label("Smoothness and motion clarity")
            .xalign(0.0)
            .css_classes(["hz-subtitle"])
            .build(),
    );
    header.append(&titles);
    header.set_valign(gtk::Align::Start);

    let current = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(2)
        .valign(gtk::Align::Center)
        .build();
    current.append(
        &gtk::Label::builder()
            .label(props.current_hz.to_string())
            .css_classes(["hz-current"])
            .build(),
    );
    current.append(
        &gtk::Label::builder()
            .label("Hz")
            .css_classes(["hz-current-unit"])
            .build(),
    );
    header.append(&current);
    root.append(&header);

    // A wrapping track: the number of advertised rates differs per display and must not
    // widen the card.
    let track = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .column_spacing(8)
        .row_spacing(8)
        .min_children_per_line(1)
        .max_children_per_line(6)
        .homogeneous(true)
        .activate_on_single_click(false)
        .css_classes(["hz-track"])
        .build();

    let enabled = props.enabled;
    for opt in &props.options {
        let pill = gtk::Button::builder()
            .label(opt.hz.to_string())
            .sensitive(enabled)
            .hexpand(true)
            .css_classes(if opt.active {
                vec!["hz-pill", "hz-pill--active"]
            } else if opt.custom {
                vec!["hz-pill", "hz-pill--custom"]
            } else {
                vec!["hz-pill"]
            })
            .build();
        if opt.custom {
            pill.set_tooltip_text(Some("Click to select · Right-click to remove"));
            let on_pick = props.on_pick.clone();
            let on_remove = props.on_remove_custom.clone();
            let hz = opt.hz;
            let gesture = gtk::GestureClick::new();
            gesture.set_button(0);
            gesture.connect_pressed(move |_, _, _, button| {
                if button == 3.0 {
                    on_remove(hz);
                } else {
                    on_pick(hz);
                }
            });
            pill.add_controller(gesture);
        } else {
            let on_pick = props.on_pick.clone();
            let hz = opt.hz;
            let active = opt.active;
            pill.connect_clicked(move |_| {
                if !active {
                    on_pick(hz);
                }
            });
        }
        track.append(&pill);
    }

    let add = gtk::Button::builder()
        .label("+")
        .tooltip_text("Add custom refresh rate")
        .sensitive(enabled)
        .hexpand(true)
        .css_classes(["hz-pill", "hz-pill--add"])
        .build();
    let popover = gtk::Popover::builder().build();
    let entry = gtk::Entry::builder()
        .placeholder_text("Hz")
        .max_width_chars(5)
        .width_chars(5)
        .input_purpose(gtk::InputPurpose::Digits)
        .css_classes(["rate-entry"])
        .build();
    let pop_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    pop_box.append(&entry);
    let confirm = gtk::Button::builder().label("Add").build();
    pop_box.append(&confirm);
    popover.set_child(Some(&pop_box));
    popover.set_parent(&add);

    let on_add = props.on_add_custom.clone();
    let submit = {
        let entry = entry.clone();
        let popover = popover.clone();
        move || {
            if let Ok(hz) = entry.text().trim().parse::<u32>() {
                entry.set_text("");
                popover.popdown();
                on_add(hz);
            }
        }
    };
    entry.connect_activate({
        let submit = submit.clone();
        move |_| submit()
    });
    confirm.connect_clicked({
        let submit = submit.clone();
        move |_| submit()
    });
    add.connect_clicked(move |_| popover.popup());

    track.append(&add);
    root.append(&track);
    root.upcast()
}
