use gtk::pango;
use gtk::prelude::*;
use gtk4 as gtk;

use super::props::{CanvasWidgets, TileProps, TileWidgets};

const DOT_SPACING: f64 = 24.0;
const CANVAS_HEIGHT: i32 = 280;

pub fn build() -> CanvasWidgets {
    let backdrop = gtk::DrawingArea::builder()
        .content_height(CANVAS_HEIGHT)
        .hexpand(true)
        .build();
    backdrop.set_draw_func(|_, cr, width, height| {
        // A dotted grid in the border colour, centred like the design's radial gradient.
        cr.set_source_rgba(0.173, 0.192, 0.227, 0.35);
        let (w, h) = (f64::from(width), f64::from(height));
        let mut x = (w / 2.0) % DOT_SPACING;
        while x < w {
            let mut y = (h / 2.0) % DOT_SPACING;
            while y < h {
                cr.arc(x, y, 1.0, 0.0, std::f64::consts::TAU);
                let _ = cr.fill();
                y += DOT_SPACING;
            }
            x += DOT_SPACING;
        }
    });

    let fixed = gtk::Fixed::new();
    let ghost = gtk::Box::builder()
        .visible(false)
        .can_target(false)
        .css_classes(["monitor-ghost"])
        .build();
    fixed.put(&ghost, 0.0, 0.0);

    let hint = gtk::Label::builder()
        .visible(false)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::End)
        .margin_start(16)
        .margin_bottom(12)
        .can_target(false)
        .css_classes(["monitor-canvas__hint"])
        .build();

    let root = gtk::Overlay::builder()
        .child(&backdrop)
        .css_classes(["monitor-canvas"])
        .overflow(gtk::Overflow::Hidden)
        .build();
    root.add_overlay(&fixed);
    root.add_overlay(&hint);

    CanvasWidgets {
        root,
        backdrop,
        fixed,
        ghost,
        hint,
    }
}

/// Text that can shrink to nothing, so the label never props its tile open.
fn shrinking_label(class: &str) -> gtk::Label {
    gtk::Label::builder()
        .ellipsize(pango::EllipsizeMode::End)
        .width_chars(0)
        .halign(gtk::Align::Center)
        .css_classes([class])
        .build()
}

pub fn tile(props: &TileProps) -> TileWidgets {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .focusable(true)
        .overflow(gtk::Overflow::Hidden)
        .build();

    let inner = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .build();
    let badge = shrinking_label("monitor-item__badge");
    badge.set_label("Focused");
    let label = shrinking_label("monitor-item__label");
    let resolution = shrinking_label("monitor-item__res");
    inner.append(&badge);
    inner.append(&label);
    inner.append(&resolution);
    root.append(&inner);

    let widgets = TileWidgets {
        connector: props.connector.clone(),
        root,
        badge,
        label,
        resolution,
    };
    update_tile(&widgets, props);
    widgets
}

/// Brings an existing tile in line with new props without rebuilding it.
pub fn update_tile(widgets: &TileWidgets, props: &TileProps) {
    widgets.label.set_label(&props.connector);
    widgets.resolution.set_label(if props.enabled {
        props.size_label.as_str()
    } else {
        "Off"
    });
    widgets.root.set_tooltip_text(Some(if props.enabled {
        "Drag to move. With the keyboard: arrows nudge, Enter applies, Escape cancels."
    } else {
        "This display is off. Select it to turn it on."
    }));
    widgets
        .root
        .set_cursor_from_name(props.enabled.then_some("grab"));
    widgets.root.set_css_classes(&tile_classes(props));
}

fn tile_classes(props: &TileProps) -> Vec<&'static str> {
    let mut classes = vec!["monitor-item"];
    if props.selected {
        classes.push("monitor-item--active");
    }
    if !props.enabled {
        classes.push("monitor-item--disabled");
    }
    classes
}
