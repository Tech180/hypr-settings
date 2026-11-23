use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use super::props::{ProfileRowProps, ProfilesGroupProps};
use crate::shared::icon::{self, Icon, IconProps};
use crate::shared::section_header::{self, HeadingSize, SectionHeaderProps};

pub fn build(props: &ProfilesGroupProps) -> gtk::Widget {
    let grid = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .column_spacing(16)
        .row_spacing(16)
        .max_children_per_line(4)
        .homogeneous(true)
        .css_classes(["profile-grid"])
        .build();
    for (index, profile) in props.rows.iter().enumerate() {
        grid.append(&build_card(profile, index, props));
    }

    let header = section_header::build(&SectionHeaderProps {
        title: "Layout Profiles".into(),
        subtitle: Some("Automated arrangements applied by kanshi".into()),
        badge: None,
        tag: None,
        size: HeadingSize::Small,
    });
    header.append(&save_active_button(props));
    header.append(&save_button(props));

    let group = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .build();
    group.append(&header);
    group.append(&grid);
    group.upcast()
}

fn build_card(profile: &ProfileRowProps, index: usize, props: &ProfilesGroupProps) -> gtk::Button {
    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .css_classes(["profile-card__header"])
        .build();
    let icon_box = gtk::CenterBox::builder()
        .css_classes(["profile-icon"])
        .build();
    icon_box.set_center_widget(Some(&icon::build(&IconProps {
        icon: profile.icon,
        size: 20,
    })));
    header.append(&icon_box);
    if profile.active {
        let badge = gtk::Label::builder()
            .label("Active")
            .halign(gtk::Align::End)
            .hexpand(true)
            .xalign(1.0)
            .css_classes(["profile-card__badge"])
            .build();
        header.append(&badge);
    }

    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .halign(gtk::Align::Start)
        .build();
    text.append(
        &gtk::Label::builder()
            .label(&profile.name)
            .xalign(0.0)
            .css_classes(["profile-card__title"])
            .build(),
    );
    text.append(
        &gtk::Label::builder()
            .label(&profile.subtitle)
            .xalign(0.0)
            .wrap(true)
            .css_classes(["profile-card__subtitle"])
            .build(),
    );

    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .build();
    column.append(&header);
    column.append(&text);

    let mut classes = vec!["profile-card"];
    if profile.active {
        classes.push("profile-card--active");
    }
    let card = gtk::Button::builder()
        .child(&column)
        .css_classes(classes)
        .build();
    let on_open = props.on_open.clone();
    card.connect_clicked(move |_| on_open(index));
    card
}

fn save_active_button(props: &ProfilesGroupProps) -> gtk::Button {
    let target = props.active_name.as_deref().unwrap_or("home");
    let button = gtk::Button::builder()
        .label("Save layout")
        .tooltip_text(format!("Store the live layout in {target}"))
        .valign(gtk::Align::End)
        .css_classes(["action-btn"])
        .build();
    let on_save_active = props.on_save_active.clone();
    button.connect_clicked(move |_| on_save_active());
    button
}

fn save_button(props: &ProfilesGroupProps) -> gtk::Button {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    content.append(&icon::build(&IconProps {
        icon: Icon::Save,
        size: 16,
    }));
    content.append(&gtk::Label::new(Some("Save current as…")));
    let button = gtk::Button::builder()
        .child(&content)
        .valign(gtk::Align::End)
        .css_classes(["action-btn"])
        .build();
    let on_save_as = props.on_save_as.clone();
    button.connect_clicked(move |button| present_save_dialog(button, on_save_as.clone()));
    button
}

fn present_save_dialog(anchor: &gtk::Button, on_save_as: std::rc::Rc<dyn Fn(String)>) {
    let entry = gtk::Entry::builder()
        .placeholder_text("Profile name")
        .activates_default(true)
        .build();
    let dialog = adw::AlertDialog::builder()
        .heading("Save current layout")
        .body("Stores the layout of all connected displays under this name.")
        .extra_child(&entry)
        .close_response("cancel")
        .default_response("save")
        .build();
    dialog.add_responses(&[("cancel", "Cancel"), ("save", "Save")]);
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    dialog.connect_response(None, move |_, response| {
        let name = entry.text().trim().to_owned();
        if response == "save" && !name.is_empty() {
            on_save_as(name);
        }
    });
    dialog.present(Some(anchor));
}
