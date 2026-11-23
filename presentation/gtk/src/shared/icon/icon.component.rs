use gtk4 as gtk;

use super::html;
use super::props::IconProps;

/// A square vector icon painted in the widget's CSS `color`.
pub fn build(props: &IconProps) -> gtk::Widget {
    html::build(props)
}
