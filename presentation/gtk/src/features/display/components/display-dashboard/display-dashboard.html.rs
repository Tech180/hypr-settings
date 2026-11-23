use gtk::prelude::*;
use gtk4 as gtk;

use super::props::{DashboardBlock, DashboardColumnProps, DisplayDashboardProps};
use crate::features::display::components::{feature_grid, hz_group};
use crate::shared::settings_group;

const STACK_WIDTH: i32 = 720;

pub fn build(props: &DisplayDashboardProps) -> gtk::Widget {
    let left = build_column(&props.left);
    let right = build_column(&props.right);
    // Equal columns keep the layout the same width whichever display is selected; the
    // natural width of each column differs per display and must not decide the split.
    let bento = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(20)
        .homogeneous(true)
        .css_classes(["display-dashboard", "bento-grid"])
        .build();
    bento.append(&left);
    bento.append(&right);

    let bento_for_notify = bento.clone();
    let left_for_notify = left.clone();
    let right_for_notify = right.clone();
    bento.connect_notify_local(Some("width"), move |widget, _| {
        let width = widget.width();
        let narrow = width > 0 && width < STACK_WIDTH;
        bento_for_notify.set_orientation(if narrow {
            gtk::Orientation::Vertical
        } else {
            gtk::Orientation::Horizontal
        });
        // Stacked columns keep their own heights; side by side they are equal halves.
        bento_for_notify.set_homogeneous(!narrow);
        left_for_notify.set_hexpand(!narrow);
        right_for_notify.set_hexpand(!narrow);
    });

    bento.upcast()
}

fn build_column(props: &DashboardColumnProps) -> gtk::Box {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .hexpand(true)
        .css_classes(["bento-col"])
        .build();
    for block in &props.blocks {
        column.append(&build_block(block));
    }
    column
}

fn build_block(block: &DashboardBlock) -> gtk::Widget {
    match block {
        DashboardBlock::Group(group) => settings_group::build(group),
        DashboardBlock::Hz(hz) => hz_group::build(hz),
        DashboardBlock::Features(grid) => feature_grid::build(grid),
    }
}
