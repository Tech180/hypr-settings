#[path = "network-actions.interface.rs"]
pub mod actions;
#[path = "network-page-props.interface.rs"]
pub mod page_props;

pub use actions::{NetPage, NetworkActions};
pub use page_props::{
    CardMemory, ConfirmProps, DiagnosticsProps, EthernetProps, HomeProps, HomeWidgets,
    PageFrame, PropertiesProps, QrProps, VpnDetailProps, VpnFormProps, VpnProps, WifiPagination,
    WifiProps,
    WifiSavedEntry,
};
pub use settings_application::network_dto::PrimaryLink;
