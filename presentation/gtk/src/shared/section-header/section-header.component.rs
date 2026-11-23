use gtk4 as gtk;

use super::html;
use super::props::SectionHeaderProps;

/// Title, optional badge and subtitle. The caller may append an action at the end.
pub fn build(props: &SectionHeaderProps) -> gtk::Box {
    html::build(props)
}
