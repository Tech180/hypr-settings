use gtk4 as gtk;

use super::html;
use super::props::IconTileProps;

/// A glyph on a rounded tile, the leading mark of rows and page heroes.
pub fn build(props: &IconTileProps) -> gtk::Widget {
    html::build(props)
}
