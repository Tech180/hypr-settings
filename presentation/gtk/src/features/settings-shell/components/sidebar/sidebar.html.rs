use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use super::props::{SidebarProps, SidebarWidgets};
use crate::shared::icon::{self, IconProps};

pub fn build(props: &SidebarProps) -> SidebarWidgets {
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["sidebar__list"])
        .build();

    for entry in &props.entries {
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .build();
        content.append(&icon::build(&IconProps {
            icon: entry.icon,
            size: 18,
        }));
        content.append(
            &gtk::Label::builder()
                .label(&entry.title)
                .xalign(0.0)
                .css_classes(["sidebar__label"])
                .build(),
        );
        let row = gtk::ListBoxRow::builder()
            .name(&entry.id)
            .child(&content)
            .selectable(entry.enabled)
            .activatable(entry.enabled)
            .css_classes(["sidebar__row"])
            .build();
        if !entry.enabled {
            row.add_css_class("sidebar__row--disabled");
        }
        list.append(&row);
    }

    let on_select = props.on_select.clone();
    list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            on_select(row.widget_name().to_string());
        }
    });

    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["sidebar"])
        .build();
    column.append(&props.search);
    column.append(&scroller);

    let title = gtk::Label::builder()
        .label(&props.title)
        .css_classes(["shell__title"])
        .build();
    let header = adw::HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .title_widget(&gtk::Box::new(gtk::Orientation::Horizontal, 0))
        .css_classes(["shell__sidebar-header"])
        .build();
    header.pack_start(&title);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&column));

    let page = adw::NavigationPage::builder()
        .title(&props.title)
        .child(&toolbar)
        .build();
    SidebarWidgets { page, list }
}
