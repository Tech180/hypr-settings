use std::rc::Rc;

use settings_application::dto::{DeviceClass, DeviceId, ProfileKind};

#[derive(Clone)]
pub struct BluetoothActions {
    pub set_powered: Rc<dyn Fn(bool)>,
    pub connect: Rc<dyn Fn(DeviceId)>,
    pub pair: Rc<dyn Fn(DeviceId)>,
    pub open_device: Rc<dyn Fn(DeviceId)>,
    pub open_advanced: Rc<dyn Fn()>,
    pub open_scan: Rc<dyn Fn()>,
    pub start_scan: Rc<dyn Fn()>,
    pub stop_scan: Rc<dyn Fn()>,
    pub close_subpage: Rc<dyn Fn()>,
    pub select_adapter: Rc<dyn Fn(String)>,
    pub select_advanced_tab: Rc<dyn Fn(usize)>,
    pub forget: Rc<dyn Fn(DeviceId)>,
    pub set_connected: Rc<dyn Fn(DeviceId, bool)>,
    pub set_class: Rc<dyn Fn(DeviceId, DeviceClass)>,
    pub set_profile: Rc<dyn Fn(DeviceId, ProfileKind, bool)>,
    pub set_codec: Rc<dyn Fn(DeviceId, String)>,
    pub set_absolute_volume: Rc<dyn Fn(DeviceId, bool)>,
    pub set_pan: Rc<dyn Fn(bool)>,
    pub set_avrcp: Rc<dyn Fn(String)>,
    pub set_sample_rate: Rc<dyn Fn(u32)>,
    pub set_offload_disabled: Rc<dyn Fn(bool)>,
    pub set_hci_snoop: Rc<dyn Fn(bool)>,
    pub clear_gatt: Rc<dyn Fn()>,
}
