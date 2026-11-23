use gtk4 as gtk;

use super::html;
use super::props::FeatureGridProps;

pub fn build(props: &FeatureGridProps) -> gtk::Widget {
    html::build(props)
}
