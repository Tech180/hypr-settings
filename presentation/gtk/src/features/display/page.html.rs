use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use super::components::{display_dashboard, profiles_group};
use super::constants::{CONTENT_GAP, SLIDE_MS};
use super::interfaces::{DisplayPageProps, PageFrame};
use super::utils::arrangement_header;
use crate::shared::section_header;

/// Main view and profile sub page side by side in a sliding stack, pill floating above.
pub fn build_frame(pill: &gtk::Widget, canvas: &gtk::Widget) -> PageFrame {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(CONTENT_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content"])
        .build();

    let arrangement = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    arrangement.append(&section_header::build(&arrangement_header()));
    arrangement.append(canvas);
    let notice = gtk::Label::builder()
        .visible(false)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["page__notice"])
        .build();
    arrangement.append(&notice);
    column.append(&arrangement);

    let rest = adw::Bin::new();
    column.append(&rest);
    let content = column.upcast();

    let body = adw::Bin::new();
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .vexpand(true)
        .child(&body)
        .build();
    let detail = adw::Bin::new();

    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::SlideLeftRight)
        .transition_duration(SLIDE_MS)
        .hexpand(true)
        .vexpand(true)
        .build();
    stack.add_named(&scroller, Some("main"));
    stack.add_named(&detail, Some("profile"));

    let root = gtk::Overlay::builder()
        .child(&stack)
        .css_classes(["page"])
        .build();
    root.add_overlay(pill);
    PageFrame {
        root,
        stack,
        scroller,
        body,
        content,
        notice,
        rest,
        detail,
    }
}

/// Everything below the arrangement: the selected output's settings and the profiles.
pub fn build_rest(props: &DisplayPageProps) -> gtk::Widget {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(CONTENT_GAP)
        .build();

    let output = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    output.append(&section_header::build(&props.output_header));
    output.append(&display_dashboard::build(&props.dashboard));
    column.append(&output);

    column.append(&profiles_group::build(&props.profiles));
    column.upcast()
}

pub fn build_empty() -> gtk::Widget {
    adw::StatusPage::builder()
        .title("No displays found")
        .description("Hyprland did not report any outputs.")
        .vexpand(true)
        .build()
        .upcast()
}
