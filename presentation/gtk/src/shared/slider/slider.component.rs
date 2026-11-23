use gtk4 as gtk;

use super::html;
use super::props::SliderProps;

/// A horizontal track with a live numeric readout.
pub fn build(props: &SliderProps) -> gtk::Widget {
    html::build(props)
}
