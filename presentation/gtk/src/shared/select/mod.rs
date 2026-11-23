#[path = "constants/select-layout.constant.rs"]
mod layout_constants;

#[path = "interfaces/select-props.interface.rs"]
mod props;

#[path = "utils/option-row.util.rs"]
mod option_row;

#[path = "select.component.rs"]
mod component;
#[path = "select.html.rs"]
mod html;

pub use component::build;
pub use props::SelectProps;
