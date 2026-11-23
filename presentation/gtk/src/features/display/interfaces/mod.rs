#[path = "display-actions.interface.rs"]
mod display_actions;
#[path = "display-page-props.interface.rs"]
mod display_page_props;

pub use display_actions::DisplayActions;
pub use display_page_props::{DisplayPageProps, PageFrame};
