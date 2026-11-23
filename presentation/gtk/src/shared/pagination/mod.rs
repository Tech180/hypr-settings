#[path = "constants/page-size.constant.rs"]
pub mod constants;

#[path = "interfaces/pagination-props.interface.rs"]
mod props;

#[path = "utils/page-window.util.rs"]
pub mod page_window;

#[path = "utils/slice-page.util.rs"]
pub mod slice;

#[path = "pagination.component.rs"]
mod component;
#[path = "pagination.html.rs"]
mod html;

pub use component::build;
pub use constants::{DEFAULT_PAGE_SIZE, PAGE_SIZE_OPTIONS};
pub use props::PaginationProps;
