use gtk::prelude::*;
use gtk4 as gtk;

use super::props::SegmentedProps;

/// A wrapping track: the number of choices differs per display (scale presets follow the
/// resolution), so it must wrap instead of widening the row and the window with it.
pub fn build(props: &SegmentedProps) -> gtk::Widget {
    let track = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .homogeneous(true)
        .column_spacing(0)
        .row_spacing(2)
        .min_children_per_line(1)
        .max_children_per_line(u32::try_from(props.labels.len().max(1)).unwrap_or(1))
        .activate_on_single_click(false)
        .halign(gtk::Align::End)
        .valign(gtk::Align::Center)
        .css_classes(["seg-control"])
        .build();
    for (index, label) in props.labels.iter().enumerate() {
        let mut classes = vec!["seg-btn"];
        if props.selected == Some(index) {
            classes.push("seg-btn--active");
        }
        let button = gtk::Button::builder()
            .label(label)
            .css_classes(classes)
            .build();
        let on_select = props.on_select.clone();
        button.connect_clicked(move |_| on_select(index));
        track.append(&button);
    }
    let mut index = 0;
    while let Some(child) = track.child_at_index(index) {
        child.set_focusable(false);
        index += 1;
    }
    track.upcast()
}
