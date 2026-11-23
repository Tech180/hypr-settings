use gtk4 as gtk;

use super::html;
use super::props::SegmentedProps;

/// A row of mutually exclusive buttons in one outlined track.
pub fn build(props: &SegmentedProps) -> gtk::Widget {
    html::build(props)
}
