use gtk::prelude::*;
use gtk4 as gtk;

use super::props::{TrialPillProps, TrialPillWidgets};
use crate::shared::icon::{self, Icon, IconProps};

pub fn build(props: &TrialPillProps) -> TrialPillWidgets {
    let message = gtk::Label::builder()
        .xalign(0.0)
        .css_classes(["trial-pill__message"])
        .build();

    let revert = gtk::Button::builder()
        .label("Revert")
        .css_classes(["trial-pill__btn", "trial-pill__revert"])
        .build();
    let on_revert = props.on_revert.clone();
    revert.connect_clicked(move |_| on_revert());

    let keep = gtk::Button::builder()
        .label("Keep changes")
        .css_classes(["trial-pill__btn", "trial-pill__keep"])
        .build();
    let on_keep = props.on_keep.clone();
    keep.connect_clicked(move |_| on_keep());

    let actions = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_start(16)
        .build();
    actions.append(&revert);
    actions.append(&keep);

    let pill = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .css_classes(["trial-pill"])
        .build();
    pill.append(&icon::build(&IconProps {
        icon: Icon::Info,
        size: 16,
    }));
    pill.append(&message);
    pill.append(&actions);

    let root = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .transition_duration(300)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Start)
        .margin_top(24)
        .can_target(false)
        .child(&pill)
        .build();
    TrialPillWidgets {
        root,
        message,
        keep,
        revert,
    }
}
