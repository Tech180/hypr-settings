use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use super::props::{ProfileDetailProps, SavedOutputProps};
use crate::shared::icon::{self, Icon, IconProps};
use crate::shared::section_header::{self, HeadingSize, SectionHeaderProps};

pub fn build(props: &ProfileDetailProps) -> gtk::Widget {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    page.append(&top_bar(props));

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(40)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "page-content--sub"])
        .build();
    content.append(&saved_outputs(props));
    content.append(&apply_button(props));

    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .vexpand(true)
        .child(&content)
        .build();
    page.append(&scroller);
    page.upcast()
}

fn top_bar(props: &ProfileDetailProps) -> gtk::Widget {
    let back_content = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    back_content.append(&icon::build(&IconProps {
        icon: Icon::ArrowLeft,
        size: 18,
    }));
    back_content.append(&gtk::Label::new(Some("Back")));
    let back = gtk::Button::builder()
        .child(&back_content)
        .css_classes(["bar-btn", "bar-btn--wide"])
        .build();
    let on_back = props.on_back.clone();
    back.connect_clicked(move |_| on_back());

    let title = gtk::Label::builder()
        .label(format!("Profile: {}", props.name))
        .hexpand(true)
        .css_classes(["shell__title"])
        .build();

    let delete = gtk::Button::builder()
        .child(&icon::build(&IconProps {
            icon: Icon::Trash,
            size: 16,
        }))
        .tooltip_text("Delete profile")
        .sensitive(props.removable)
        .css_classes(["bar-btn", "bar-btn--danger"])
        .build();
    let on_delete = props.on_delete.clone();
    delete.connect_clicked(move |button| confirm_delete(button, &on_delete));

    let bar = gtk::CenterBox::builder()
        .css_classes(["profile-detail__bar"])
        .build();
    bar.set_start_widget(Some(&back));
    bar.set_center_widget(Some(&title));
    bar.set_end_widget(Some(&delete));
    gtk::WindowHandle::builder().child(&bar).build().upcast()
}

fn confirm_delete(anchor: &gtk::Button, on_delete: &std::rc::Rc<dyn Fn()>) {
    let dialog = adw::AlertDialog::builder()
        .heading("Delete this profile?")
        .body("kanshi will stop applying it. Other profiles are not affected.")
        .close_response("cancel")
        .build();
    dialog.add_responses(&[("cancel", "Cancel"), ("delete", "Delete")]);
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    let on_delete = on_delete.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "delete" {
            on_delete();
        }
    });
    dialog.present(Some(anchor));
}

fn saved_outputs(props: &ProfileDetailProps) -> gtk::Widget {
    let group = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["boxed-group"])
        .build();
    for output in &props.outputs {
        group.append(&saved_row(output));
    }

    let section = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    section.append(&section_header::build(&SectionHeaderProps {
        title: "Saved Outputs".into(),
        subtitle: Some(props.subtitle.clone()),
        badge: None,
        tag: None,
        size: HeadingSize::Large,
    }));
    section.append(&group);
    section.upcast()
}

fn saved_row(output: &SavedOutputProps) -> gtk::Widget {
    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    text.append(
        &gtk::Label::builder()
            .label(&output.title)
            .xalign(0.0)
            .wrap(true)
            .css_classes(["boxed-row__title"])
            .build(),
    );
    if !output.subtitle.is_empty() {
        text.append(
            &gtk::Label::builder()
                .label(&output.subtitle)
                .xalign(0.0)
                .wrap(true)
                .css_classes(["boxed-row__subtitle"])
                .build(),
        );
    }
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .css_classes(["boxed-row"])
        .build();
    row.append(&text);
    if let Some(badge) = &output.badge {
        row.append(
            &gtk::Label::builder()
                .label(badge)
                .valign(gtk::Align::Center)
                .css_classes(["hs-badge"])
                .build(),
        );
    }
    row.upcast()
}

fn apply_button(props: &ProfileDetailProps) -> gtk::Widget {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::Center)
        .build();
    content.append(&icon::build(&IconProps {
        icon: Icon::CircleCheck,
        size: 18,
    }));
    content.append(&gtk::Label::new(Some("Apply Profile Now")));
    let button = gtk::Button::builder()
        .child(&content)
        .css_classes(["action-btn", "action-btn--primary"])
        .build();
    let on_apply = props.on_apply.clone();
    button.connect_clicked(move |_| on_apply());
    button.upcast()
}
