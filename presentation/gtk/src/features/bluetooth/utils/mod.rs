#[path = "detail-rows.util.rs"]
mod detail_rows;
#[path = "page-props.util.rs"]
pub mod page_props;

pub use detail_rows::{auracast_row, developer_rows, pan_row, present_forget_dialog, profile_rows};
pub use page_props::{advanced_props, device_detail, filter_devices, main_props, scan_props};
