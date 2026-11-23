use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use crate::shared::icon::{self, Icon, IconProps};

/// A fixed back bar over a scrolling body, so long subpages scroll instead of growing the
/// window.
pub fn frame(title: &str, on_back: Rc<dyn Fn()>, content: &gtk::Widget) -> gtk::Widget {
    frame_with_end(title, on_back, None, content)
}

pub fn frame_with_end(
    title: &str,
    on_back: Rc<dyn Fn()>,
    end: Option<&gtk::Widget>,
    content: &gtk::Widget,
) -> gtk::Widget {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    page.append(&bar(title, on_back, end));
    page.append(
        &gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .hexpand(true)
            .halign(gtk::Align::Fill)
            .vexpand(true)
            .child(content)
            .build(),
    );
    page.upcast()
}

fn bar(title: &str, on_back: Rc<dyn Fn()>, end: Option<&gtk::Widget>) -> gtk::Widget {
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
        .tooltip_text("Back (Esc)")
        .css_classes(["bar-btn", "bar-btn--wide"])
        .build();
    back.connect_clicked(move |_| on_back());

    let title = gtk::Label::builder()
        .label(title)
        .hexpand(true)
        .css_classes(["shell__title"])
        .build();

    let bar = gtk::CenterBox::builder()
        .css_classes(["profile-detail__bar"])
        .build();
    bar.set_start_widget(Some(&back));
    bar.set_center_widget(Some(&title));
    if let Some(end) = end {
        bar.set_end_widget(Some(end));
    }
    gtk::WindowHandle::builder().child(&bar).build().upcast()
}
