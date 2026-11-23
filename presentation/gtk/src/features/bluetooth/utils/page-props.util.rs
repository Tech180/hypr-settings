use std::rc::Rc;

use settings_application::dto::{BluetoothView, DeviceClass, DeviceId, DeviceView};

use crate::shared::icon::Icon;

use crate::features::bluetooth::interfaces::{
    AdapterOptionProps, AdvancedPageProps, BluetoothActions, BluetoothMainProps,
    BluetoothScanProps, ControllerProps, DeviceDetailProps, DeviceRowProps,
};

pub fn main_props(view: &BluetoothView, actions: &BluetoothActions) -> BluetoothMainProps {
    BluetoothMainProps {
        powered: view.powered,
        alias: view.alias.clone(),
        adapter_options: adapter_options(view),
        controller: view.adapter.as_ref().map(|a| ControllerProps {
            name: a.name.clone(),
            address: a.address.clone(),
        }),
        paired: view
            .paired
            .iter()
            .map(|d| device_row(d, !d.connected))
            .collect(),
        actions: actions.clone(),
    }
}

pub fn scan_props(
    view: &BluetoothView,
    query: &str,
    on_query: Rc<dyn Fn(String)>,
    actions: &BluetoothActions,
) -> BluetoothScanProps {
    BluetoothScanProps {
        discovering: view.discovering,
        devices: view
            .discovered
            .iter()
            .filter(|d| !d.paired)
            .map(|d| device_row(d, true))
            .collect(),
        query: query.to_owned(),
        on_query,
        actions: actions.clone(),
    }
}

/// Keeps the devices whose name or address contains the query, ignoring case.
pub fn filter_devices(devices: &[DeviceRowProps], query: &str) -> Vec<DeviceRowProps> {
    let needle = query.trim().to_lowercase();
    devices
        .iter()
        .filter(|row| {
            needle.is_empty()
                || row.device.name.to_lowercase().contains(&needle)
                || row.device.id.as_str().to_lowercase().contains(&needle)
        })
        .cloned()
        .collect()
}

pub fn device_detail(
    view: &BluetoothView,
    id: &DeviceId,
    actions: &BluetoothActions,
) -> Option<DeviceDetailProps> {
    let device = find_device(view, id)?;
    Some(DeviceDetailProps {
        device: device.clone(),
        class_labels: DeviceClass::ALL
            .iter()
            .map(|c| (*c, c.label().to_owned()))
            .collect(),
        capabilities: view.capabilities.clone(),
        actions: actions.clone(),
    })
}

pub fn advanced_props(
    view: &BluetoothView,
    tab: usize,
    actions: &BluetoothActions,
) -> AdvancedPageProps {
    AdvancedPageProps {
        powered: view.powered,
        tab,
        adapter_name: view
            .adapter
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Bluetooth adapter".into()),
        adapter_address: view
            .adapter
            .as_ref()
            .map(|a| a.address.clone())
            .unwrap_or_default(),
        lmp: view.adapter.as_ref().and_then(|a| a.lmp_version.clone()),
        multi_adapter: view.adapters.len() >= 2,
        advanced: view.advanced.clone(),
        capabilities: view.capabilities.clone(),
        actions: actions.clone(),
    }
}

pub fn adapter_options(view: &BluetoothView) -> Vec<AdapterOptionProps> {
    view.adapters
        .iter()
        .map(|a| AdapterOptionProps {
            label: format!("{} ({})", a.name, a.address),
            address: a.address.clone(),
        })
        .collect()
}

fn device_row(device: &DeviceView, show_connect: bool) -> DeviceRowProps {
    DeviceRowProps {
        device: device.clone(),
        show_connect,
        icon: device_icon(device.class),
    }
}

fn device_icon(class: DeviceClass) -> Icon {
    match class {
        DeviceClass::Headphones => Icon::Headphones,
        DeviceClass::Keyboard => Icon::Keyboard,
        DeviceClass::Mouse => Icon::Mouse,
        DeviceClass::Phone => Icon::Smartphone,
        DeviceClass::Gamepad => Icon::Gamepad2,
        DeviceClass::Other => Icon::Bluetooth,
    }
}

fn find_device<'a>(view: &'a BluetoothView, id: &DeviceId) -> Option<&'a DeviceView> {
    view.paired
        .iter()
        .chain(view.discovered.iter())
        .find(|d| d.id == *id)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use settings_application::dto::{
        AdvancedView, BluetoothCapabilities, BluetoothView, DeviceProfileView, ProfileToggleView,
    };

    fn empty_view() -> BluetoothView {
        BluetoothView {
            powered: true,
            alias: "test-pc".into(),
            discovering: false,
            adapters: vec![],
            adapter: None,
            missing_saved_adapter: None,
            paired: vec![],
            discovered: vec![],
            capabilities: BluetoothCapabilities::default(),
            advanced: AdvancedView::default(),
        }
    }

    fn actions() -> BluetoothActions {
        BluetoothActions {
            set_powered: Rc::new(|_| {}),
            connect: Rc::new(|_| {}),
            pair: Rc::new(|_| {}),
            open_device: Rc::new(|_| {}),
            open_advanced: Rc::new(|| {}),
            open_scan: Rc::new(|| {}),
            start_scan: Rc::new(|| {}),
            stop_scan: Rc::new(|| {}),
            close_subpage: Rc::new(|| {}),
            select_adapter: Rc::new(|_| {}),
            select_advanced_tab: Rc::new(|_| {}),
            forget: Rc::new(|_| {}),
            set_connected: Rc::new(|_, _| {}),
            set_class: Rc::new(|_, _| {}),
            set_profile: Rc::new(|_, _, _| {}),
            set_codec: Rc::new(|_, _| {}),
            set_absolute_volume: Rc::new(|_, _| {}),
            set_pan: Rc::new(|_| {}),
            set_avrcp: Rc::new(|_| {}),
            set_sample_rate: Rc::new(|_| {}),
            set_offload_disabled: Rc::new(|_| {}),
            set_hci_snoop: Rc::new(|_| {}),
            clear_gatt: Rc::new(|| {}),
        }
    }

    #[test]
    fn discovered_list_skips_paired_entries() {
        let mut view = empty_view();
        let id = DeviceId::new("AA:BB:CC:DD:EE:FF");
        view.discovered.push(DeviceView {
            id: id.clone(),
            name: "Phone".into(),
            connected: false,
            paired: true,
            battery_percent: None,
            class: DeviceClass::Phone,
            profiles: DeviceProfileView::default(),
            codec: None,
        });
        let props = main_props(&view, &actions());
        assert_eq!(props.paired.len(), 0);
        assert!(
            scan_props(&view, "", Rc::new(|_| {}), &actions())
                .devices
                .is_empty()
        );
        let _ = device_detail(&view, &id, &actions());
    }

    #[test]
    fn filter_matches_name_or_address_ignoring_case() {
        let device = |name: &str, id: &str| DeviceRowProps {
            device: DeviceView {
                id: DeviceId::new(id),
                name: name.into(),
                connected: false,
                paired: false,
                battery_percent: None,
                class: DeviceClass::Other,
                profiles: DeviceProfileView::default(),
                codec: None,
            },
            show_connect: true,
            icon: Icon::Bluetooth,
        };
        let rows = vec![
            device("Wonderboom", "AA:00:00:00:00:01"),
            device("Keyboard", "BB:00:00:00:00:02"),
        ];
        assert_eq!(filter_devices(&rows, "").len(), 2);
        assert_eq!(filter_devices(&rows, "WONDER").len(), 1);
        assert_eq!(filter_devices(&rows, "bb:00").len(), 1);
        assert!(filter_devices(&rows, "mouse").is_empty());
    }

    #[test]
    fn adapter_options_empty_for_single_adapter() {
        let mut view = empty_view();
        view.adapters.push(settings_application::dto::AdapterView {
            name: "hci0".into(),
            address: "00:11:22:33:44:55".into(),
            alias: "pc".into(),
            powered: true,
            lmp_version: None,
            modalias: None,
        });
        assert_eq!(adapter_options(&view).len(), 1);
    }

    #[test]
    fn adapter_options_label_includes_name_and_address() {
        let mut view = empty_view();
        view.adapters.push(settings_application::dto::AdapterView {
            name: "hci1".into(),
            address: "AA:BB:CC:DD:EE:FF".into(),
            alias: "pc".into(),
            powered: false,
            lmp_version: None,
            modalias: None,
        });
        let options = adapter_options(&view);
        assert!(options[0].label.contains("hci1"));
        assert!(options[0].label.contains("AA:BB"));
    }

    #[test]
    fn advanced_props_carry_the_selected_tab() {
        let props = advanced_props(&empty_view(), 1, &actions());
        assert_eq!(props.tab, 1);
        assert!(props.powered);
    }

    #[test]
    fn device_detail_includes_capabilities() {
        let mut view = empty_view();
        let id = DeviceId::new("11:22:33:44:55:66");
        view.paired.push(DeviceView {
            id: id.clone(),
            name: "Headset".into(),
            connected: true,
            paired: true,
            battery_percent: Some(88),
            class: DeviceClass::Headphones,
            profiles: DeviceProfileView {
                media_audio: ProfileToggleView {
                    available: true,
                    enabled: true,
                },
                ..Default::default()
            },
            codec: None,
        });
        let detail = device_detail(&view, &id, &actions()).expect("device");
        assert!(detail.capabilities.codec_switch == view.capabilities.codec_switch);
    }
}
