use gtk4 as gtk;
use libadwaita as adw;
use settings_application::dto::{AdvancedView, BluetoothCapabilities, DeviceClass, DeviceView};

use crate::shared::icon::Icon;

use super::actions::BluetoothActions;

#[derive(Clone)]
pub struct AdapterOptionProps {
    pub label: String,
    pub address: String,
}

#[derive(Clone)]
pub struct BluetoothMainProps {
    pub powered: bool,
    pub alias: String,
    pub adapter_options: Vec<AdapterOptionProps>,
    pub controller: Option<ControllerProps>,
    pub paired: Vec<DeviceRowProps>,
    pub actions: BluetoothActions,
}

/// The "Other Devices" subpage: a scan the user starts and stops, and what it found.
#[derive(Clone)]
pub struct BluetoothScanProps {
    pub discovering: bool,
    pub devices: Vec<DeviceRowProps>,
    /// The search text, kept by the page across rebuilds.
    pub query: String,
    pub on_query: std::rc::Rc<dyn Fn(String)>,
    pub actions: BluetoothActions,
}

/// The Bluetooth adapter as the system summary shows it.
#[derive(Clone)]
pub struct ControllerProps {
    pub name: String,
    pub address: String,
}

#[derive(Clone)]
pub struct DeviceRowProps {
    pub device: DeviceView,
    pub show_connect: bool,
    pub icon: Icon,
}

#[derive(Clone)]
pub struct DeviceDetailProps {
    pub device: DeviceView,
    pub class_labels: Vec<(DeviceClass, String)>,
    pub capabilities: BluetoothCapabilities,
    pub actions: BluetoothActions,
}

#[derive(Clone)]
pub struct AdvancedPageProps {
    pub powered: bool,
    /// Index of the tab shown under the controller card.
    pub tab: usize,
    pub adapter_name: String,
    pub adapter_address: String,
    pub multi_adapter: bool,
    pub lmp: Option<String>,
    pub advanced: AdvancedView,
    pub capabilities: BluetoothCapabilities,
    pub actions: BluetoothActions,
}

/// Sliding stack shell: main list, device detail, and advanced subpages.
pub struct PageFrame {
    pub root: gtk::Stack,
    pub main_scroller: gtk::ScrolledWindow,
    pub main_body: adw::Bin,
    pub device: adw::Bin,
    pub advanced: adw::Bin,
    pub scan: adw::Bin,
}
