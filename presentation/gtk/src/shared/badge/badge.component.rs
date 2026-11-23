use gtk4 as gtk;

use super::html;
use super::props::{BadgeProps, Tone};

/// A pill with a short fact in it.
pub fn build(props: &BadgeProps) -> gtk::Widget {
    html::build(props)
}

/// A tiny filled circle that marks a live state next to a name.
pub fn dot(tone: Tone) -> gtk::Widget {
    html::dot(tone)
}
