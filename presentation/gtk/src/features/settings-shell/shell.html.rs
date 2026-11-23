use gtk4 as gtk;
use libadwaita as adw;

use super::interfaces::{ShellProps, ShellWidgets};

pub fn build(props: &ShellProps) -> ShellWidgets {
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .hexpand(true)
        .vexpand(true)
        .css_classes(["shell__content"])
        .build();
    for (id, page) in &props.pages {
        stack.add_named(page, Some(id));
    }

    // No header bar: the compositor manages the window, so there are no window controls,
    // and nothing sits on top of the page's floating trial pill.
    let content_page = adw::NavigationPage::builder()
        .title("Settings")
        .child(&stack)
        .build();

    let split = adw::NavigationSplitView::builder()
        .sidebar(&props.sidebar)
        .content(&content_page)
        .min_sidebar_width(260.0)
        .max_sidebar_width(260.0)
        .build();
    props.toasts.set_child(Some(&split));

    let window = adw::ApplicationWindow::builder()
        .application(&props.app)
        .title("Settings")
        .default_width(1100)
        .default_height(760)
        .content(&props.toasts)
        .css_classes(["shell"])
        .build();
    ShellWidgets {
        window,
        stack,
        split,
        content_page,
    }
}
