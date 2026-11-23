use gtk::prelude::*;
use gtk4 as gtk;
use settings_domain::network::{LogEntry, LogLevel};

use crate::features::network::components::common;
use crate::features::network::constants::PAGE_GAP;
use crate::features::network::interfaces::DiagnosticsProps;
use crate::features::network::utils::{clock_text, journal_text};
use crate::shared::button::ButtonKind;
use crate::shared::icon::Icon;
use crate::shared::subpage_bar;

pub fn build(props: &DiagnosticsProps) -> gtk::Widget {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "page-content--sub", "network-page"])
        .build();
    let clear = common::action(
        "Clear log",
        ButtonKind::Hollow,
        Some(Icon::Trash),
        props.actions.clear_log.clone(),
    );
    column.append(&common::section(
        "Actions",
        Some(&clear),
        &terminal(&action_lines(&props.view.actions), "No actions yet."),
    ));
    column.append(&common::section(
        "NetworkManager journal",
        None,
        &terminal(&journal_lines(&props.view.journal), "The journal is empty."),
    ));
    subpage_bar::frame(
        "Diagnostics",
        props.actions.back.clone(),
        column.upcast_ref(),
    )
}

fn action_lines(entries: &[LogEntry]) -> Vec<gtk::Widget> {
    entries
        .iter()
        .map(|entry| {
            let level = match entry.level {
                LogLevel::Info => ("term-line--info", "INFO"),
                LogLevel::Warn => ("term-line--warn", "WARN"),
                LogLevel::Error => ("term-line--error", "ERROR"),
            };
            line(&clock_text(entry.unix_secs), level.1, &entry.text, level.0)
        })
        .collect()
}

fn journal_lines(lines: &[String]) -> Vec<gtk::Widget> {
    lines
        .iter()
        .map(|raw| {
            let (time, message) = journal_text(raw);
            line(&time, "", &message, "term-line--journal")
        })
        .collect()
}

fn line(time: &str, tag: &str, text: &str, class: &str) -> gtk::Widget {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .css_classes(["term-line", class])
        .build();
    row.append(
        &gtk::Label::builder()
            .label(time)
            .valign(gtk::Align::Start)
            .css_classes(["term-line__time"])
            .build(),
    );
    if !tag.is_empty() {
        row.append(
            &gtk::Label::builder()
                .label(tag)
                .valign(gtk::Align::Start)
                .width_chars(5)
                .xalign(0.0)
                .css_classes(["term-line__tag"])
                .build(),
        );
    }
    row.append(
        &gtk::Label::builder()
            .label(text)
            .xalign(0.0)
            .hexpand(true)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::WordChar)
            .selectable(true)
            .css_classes(["term-line__text"])
            .build(),
    );
    row.upcast()
}

/// A dark, monospace log that opens scrolled to its newest line.
fn terminal(lines: &[gtk::Widget], empty: &str) -> gtk::Widget {
    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .css_classes(["terminal__body"])
        .build();
    if lines.is_empty() {
        body.append(
            &gtk::Label::builder()
                .label(empty)
                .xalign(0.0)
                .css_classes(["term-line__text", "term-line--journal"])
                .build(),
        );
    }
    for widget in lines {
        body.append(widget);
    }
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(220)
        .max_content_height(360)
        .propagate_natural_height(true)
        .child(&body)
        .css_classes(["terminal"])
        .build();
    let adjustment = scroller.vadjustment();
    let pinned = std::rc::Rc::new(std::cell::Cell::new(true));
    adjustment.connect_changed({
        let pinned = pinned.clone();
        move |adjustment| {
            if pinned.get() {
                adjustment.set_value(adjustment.upper() - adjustment.page_size());
            }
        }
    });
    // Once the user scrolls up, new content must not drag the view back down.
    adjustment.connect_value_changed(move |adjustment| {
        pinned.set(adjustment.upper() - adjustment.page_size() - adjustment.value() < 4.0);
    });
    scroller.upcast()
}
