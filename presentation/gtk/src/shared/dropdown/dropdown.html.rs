use gtk4 as gtk;

use crate::shared::select::{self, SelectProps};

use super::props::DropdownProps;

pub fn build(props: &DropdownProps) -> gtk::Widget {
    select::build(&SelectProps {
        options: props.options.clone(),
        selected: props.selected,
        on_select: props.on_select.clone(),
        placeholder: None,
        header_label: None,
        min_width: None,
    })
}
