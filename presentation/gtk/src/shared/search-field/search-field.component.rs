use gtk4 as gtk;

use super::html;
use super::props::SearchFieldProps;

/// A text field that reports the query as the user types.
pub fn build(props: &SearchFieldProps) -> gtk::SearchEntry {
    html::build(props)
}
