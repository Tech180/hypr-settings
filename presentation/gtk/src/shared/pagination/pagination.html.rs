use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use super::props::PaginationProps;
use super::page_window::{PageItem, page_window};
use crate::shared::button::{self, ButtonKind, ButtonProps, ButtonSize};
use crate::shared::select::{self, SelectProps};
use crate::shared::icon::{self, Icon, IconProps};

pub fn build(props: &PaginationProps) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .hexpand(true)
        .accessible_role(gtk::AccessibleRole::Navigation)
        .css_classes(["pagination"])
        .build();
    let label = props
        .accessible_label
        .as_deref()
        .unwrap_or("Pagination");
    root.update_property(&[gtk::accessible::Property::Label(label)]);

    root.append(&page_size_row(props));

    if props.total_pages > 1 {
        root.append(&nav_strip(props));
    }

    root.upcast()
}

fn page_size_row(props: &PaginationProps) -> gtk::Widget {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .valign(gtk::Align::Center)
        .build();
    row.append(
        &gtk::Label::builder()
            .label("Per page")
            .css_classes(["pagination__size"])
            .build(),
    );
    let labels: Vec<String> = props
        .page_size_options
        .iter()
        .map(|n| n.to_string())
        .collect();
    let selected = props
        .page_size_options
        .iter()
        .position(|&n| n == props.page_size)
        .unwrap_or(0);
    let sizes = props.page_size_options.clone();
    let on_page_size = props.on_page_size.clone();
    row.append(&select::build(&SelectProps {
        options: labels,
        selected,
        on_select: Rc::new(move |index| {
            if let Some(&size) = sizes.get(index) {
                on_page_size(size);
            }
        }),
        placeholder: None,
        header_label: None,
        min_width: Some(100),
    }));
    row.upcast()
}

fn nav_strip(props: &PaginationProps) -> gtk::Widget {
    let nav = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(2)
        .hexpand(true)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .css_classes(["pagination__nav"])
        .build();

    let page = props.page;
    let total = props.total_pages;
    let on_page = props.on_page.clone();

    let prev = button::build(
        &ButtonProps::new("Previous", ButtonKind::Ghost, {
            let on_page = on_page.clone();
            Rc::new(move || on_page(page.saturating_sub(1).max(1)))
        })
        .size(ButtonSize::Compact)
        .icon(Icon::ArrowLeft),
    );
    prev.add_css_class("pagination__prev");
    prev.set_sensitive(page > 1);
    nav.append(&prev);

    let current = u32::try_from(page.max(1)).unwrap_or(1);
    let total_u = u32::try_from(total.max(1)).unwrap_or(1);
    for item in page_window(current, total_u) {
        match item {
            PageItem::Number(n) => {
                let n_usize = usize::try_from(n).unwrap_or(1);
                let active = n_usize == page;
                let mut classes = vec!["pagination__page"];
                if active {
                    classes.push("pagination__page--active");
                }
                let button = gtk::Button::builder()
                    .label(n.to_string())
                    .css_classes(classes)
                    .sensitive(!active)
                    .build();
                if active {
                    button.update_property(&[gtk::accessible::Property::Label(&format!(
                        "Page {n}, current"
                    ))]);
                }
                let on_page = on_page.clone();
                button.connect_clicked(move |_| on_page(n_usize));
                nav.append(&button);
            }
            PageItem::Ellipsis => {
                let ell = gtk::CenterBox::builder()
                    .width_request(36)
                    .height_request(36)
                    .css_classes(["pagination__ellipsis"])
                    .build();
                ell.set_center_widget(Some(&icon::build(&IconProps {
                    icon: Icon::Ellipsis,
                    size: 18,
                })));
                ell.set_focusable(false);
                nav.append(&ell);
            }
        }
    }

    let next = button::build(
        &ButtonProps::new("Next", ButtonKind::Ghost, {
            let on_page = on_page.clone();
            Rc::new(move || on_page((page + 1).min(total)))
        })
        .size(ButtonSize::Compact)
        .icon(Icon::ArrowRight),
    );
    next.add_css_class("pagination__next");
    next.set_sensitive(page < total);
    nav.append(&next);

    nav.upcast()
}
