#[path = "bluetooth-actions.interface.rs"]
pub mod actions;
#[path = "bluetooth-page-props.interface.rs"]
pub mod page_props;

pub use actions::BluetoothActions;
pub use page_props::{
    AdapterOptionProps, AdvancedPageProps, BluetoothMainProps, BluetoothScanProps, ControllerProps,
    DeviceDetailProps, DeviceRowProps, PageFrame,
};
