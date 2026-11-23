use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use settings_application::BluetoothPorts;
use settings_application::dto::BluetoothView;
use settings_application::{BluetoothApi, BluetoothService};
use settings_domain::PortError;
use settings_domain::bluetooth::{
    AdapterSnapshot, BluetoothCapabilities, BluetoothPreferences, DeviceId, DeviceSnapshot,
    ProfileKind, ProfileSnapshot, ProfileState, pick_adapter,
};
use settings_domain::ports::{
    BluetoothEvents, BluetoothExtras, BluetoothGateway, BluetoothPreferencesStore, Subscription,
};

struct FakeGateway {
    adapters: Vec<AdapterSnapshot>,
    wanted: Mutex<Option<String>>,
    devices: HashMap<String, Vec<DeviceSnapshot>>,
}

impl FakeGateway {
    fn active(&self) -> Option<AdapterSnapshot> {
        let wanted = self.wanted.lock().unwrap().clone();
        let index = pick_adapter(&self.adapters, wanted.as_deref())?;
        self.adapters.get(index).cloned()
    }
}

impl BluetoothGateway for FakeGateway {
    fn adapters(&self) -> Result<Vec<AdapterSnapshot>, PortError> {
        Ok(self.adapters.clone())
    }

    fn set_active_adapter(&self, address: Option<&str>) -> Result<(), PortError> {
        *self.wanted.lock().unwrap() = address.map(str::to_owned);
        Ok(())
    }

    fn adapter(&self) -> Result<Option<AdapterSnapshot>, PortError> {
        Ok(self.active())
    }

    fn devices(&self) -> Result<Vec<DeviceSnapshot>, PortError> {
        let active = self
            .active()
            .ok_or_else(|| PortError::Unavailable("no adapter".into()))?;
        Ok(self
            .devices
            .get(&active.address)
            .cloned()
            .unwrap_or_default())
    }

    fn set_powered(&self, on: bool) -> Result<(), PortError> {
        let _ = on;
        Ok(())
    }

    fn set_discoverable(&self, _: bool) -> Result<(), PortError> {
        Ok(())
    }

    fn start_discovery(&self) -> Result<(), PortError> {
        Ok(())
    }

    fn stop_discovery(&self) -> Result<(), PortError> {
        Ok(())
    }

    fn pair(&self, _: &DeviceId) -> Result<(), PortError> {
        Ok(())
    }

    fn trust(&self, _: &DeviceId, _: bool) -> Result<(), PortError> {
        Ok(())
    }

    fn connect(&self, _: &DeviceId) -> Result<(), PortError> {
        Ok(())
    }

    fn disconnect(&self, _: &DeviceId) -> Result<(), PortError> {
        Ok(())
    }

    fn remove(&self, _: &DeviceId) -> Result<(), PortError> {
        Ok(())
    }

    fn set_profile(&self, _: &DeviceId, _: ProfileKind, _: bool) -> Result<(), PortError> {
        Ok(())
    }

    fn set_codec(
        &self,
        _: &DeviceId,
        _: &settings_domain::bluetooth::CodecId,
    ) -> Result<(), PortError> {
        Ok(())
    }

    fn set_absolute_volume(&self, _: &DeviceId, _: bool) -> Result<(), PortError> {
        Ok(())
    }

    fn capabilities(&self) -> BluetoothCapabilities {
        BluetoothCapabilities::default()
    }
}

struct FakeEvents;

impl BluetoothEvents for FakeEvents {
    fn subscribe(&self, _: Box<dyn Fn() + Send>) -> Result<Subscription, PortError> {
        Ok(Subscription::new(|| {}))
    }
}

struct FakeExtras;

impl BluetoothExtras for FakeExtras {
    fn pan_enabled(&self) -> Result<bool, PortError> {
        Ok(false)
    }
    fn set_pan_enabled(&self, _: bool) -> Result<(), PortError> {
        Ok(())
    }
    fn avrcp_versions(&self) -> Result<Vec<String>, PortError> {
        Ok(vec!["AVRCP 1.6".into()])
    }
    fn avrcp_version(&self) -> Result<Option<String>, PortError> {
        Ok(None)
    }
    fn set_avrcp_version(&self, _: &str) -> Result<(), PortError> {
        Ok(())
    }
    fn sample_rates(&self) -> Result<Vec<u32>, PortError> {
        Ok(vec![48_000])
    }
    fn sample_rate_hz(&self) -> Result<Option<u32>, PortError> {
        Ok(None)
    }
    fn set_sample_rate_hz(&self, _: u32) -> Result<(), PortError> {
        Ok(())
    }
    fn a2dp_offload_disabled(&self) -> Result<bool, PortError> {
        Ok(false)
    }
    fn set_a2dp_offload_disabled(&self, _: bool) -> Result<(), PortError> {
        Ok(())
    }
    fn hci_snoop_enabled(&self) -> Result<bool, PortError> {
        Ok(false)
    }
    fn set_hci_snoop_enabled(&self, _: bool) -> Result<(), PortError> {
        Ok(())
    }
    fn clear_gatt_cache(&self) -> Result<(), PortError> {
        Ok(())
    }
    fn auracast_available(&self) -> bool {
        false
    }
}

struct MemPrefs(Mutex<BluetoothPreferences>);

impl BluetoothPreferencesStore for MemPrefs {
    fn load(&self) -> Result<BluetoothPreferences, PortError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn save(&self, prefs: &BluetoothPreferences) -> Result<(), PortError> {
        *self.0.lock().unwrap() = prefs.clone();
        Ok(())
    }
}

fn adapter(name: &str, address: &str) -> AdapterSnapshot {
    AdapterSnapshot {
        name: name.into(),
        address: address.into(),
        alias: "test-pc".into(),
        powered: true,
        discoverable: true,
        discovering: false,
        lmp_version: Some("5.3".into()),
        modalias: None,
    }
}

fn buds() -> DeviceSnapshot {
    DeviceSnapshot {
        id: DeviceId::new("AA:BB:CC:DD:EE:01"),
        name: "Test Buds".into(),
        address: "AA:BB:CC:DD:EE:01".into(),
        paired: true,
        connected: true,
        trusted: true,
        battery_percent: Some(70),
        uuids: vec!["0000110b-0000-1000-8000-00805f9b34fb".into()],
        class: settings_domain::bluetooth::DeviceClass::Headphones,
        profiles: ProfileSnapshot {
            media_audio: ProfileState {
                available: true,
                enabled: true,
            },
            ..Default::default()
        },
        codec: None,
    }
}

fn keyboard() -> DeviceSnapshot {
    DeviceSnapshot {
        id: DeviceId::new("BB:CC:DD:EE:FF:01"),
        name: "Test Keyboard".into(),
        address: "BB:CC:DD:EE:FF:01".into(),
        paired: true,
        connected: false,
        trusted: true,
        battery_percent: None,
        uuids: vec![],
        class: settings_domain::bluetooth::DeviceClass::Keyboard,
        profiles: ProfileSnapshot::default(),
        codec: None,
    }
}

fn service_with_prefs(prefs: Arc<MemPrefs>) -> BluetoothService {
    let mut devices = HashMap::new();
    devices.insert("00:11:22:33:44:55".into(), vec![buds()]);
    devices.insert("11:22:33:44:55:66".into(), vec![keyboard()]);
    BluetoothService::new(BluetoothPorts {
        gateway: Arc::new(FakeGateway {
            adapters: vec![
                adapter("hci0", "00:11:22:33:44:55"),
                adapter("hci1", "11:22:33:44:55:66"),
            ],
            wanted: Mutex::new(None),
            devices,
        }),
        events: Arc::new(FakeEvents),
        extras: Arc::new(FakeExtras),
        prefs,
    })
}

fn service() -> BluetoothService {
    service_with_prefs(Arc::new(MemPrefs(Mutex::new(
        BluetoothPreferences::default(),
    ))))
}

#[test]
fn bluetooth_view_lists_paired_device() {
    let view: BluetoothView = service().bluetooth_view().unwrap();
    assert!(view.powered);
    assert_eq!(view.adapters.len(), 2);
    assert_eq!(view.paired.len(), 1);
    assert_eq!(view.paired[0].name, "Test Buds");
    assert!(view.discovered.is_empty());
}

#[test]
fn select_adapter_switches_device_list_and_persists() {
    let prefs = Arc::new(MemPrefs(Mutex::new(BluetoothPreferences::default())));
    let service = service_with_prefs(Arc::clone(&prefs));
    service
        .select_bluetooth_adapter("11:22:33:44:55:66".into())
        .unwrap();
    let view = service.bluetooth_view().unwrap();
    assert_eq!(view.paired.len(), 1);
    assert_eq!(view.paired[0].name, "Test Keyboard");
    assert_eq!(
        prefs.load().unwrap().selected_adapter.as_deref(),
        Some("11:22:33:44:55:66")
    );
}

#[test]
fn stale_saved_adapter_reports_missing() {
    let prefs = Arc::new(MemPrefs(Mutex::new(BluetoothPreferences {
        selected_adapter: Some("DE:AD:BE:EF:00:00".into()),
        ..Default::default()
    })));
    let service = BluetoothService::new(BluetoothPorts {
        gateway: Arc::new(FakeGateway {
            adapters: vec![adapter("hci0", "00:11:22:33:44:55")],
            wanted: Mutex::new(None),
            devices: HashMap::new(),
        }),
        events: Arc::new(FakeEvents),
        extras: Arc::new(FakeExtras),
        prefs,
    });
    let view = service.bluetooth_view().unwrap();
    assert_eq!(
        view.missing_saved_adapter.as_deref(),
        Some("DE:AD:BE:EF:00:00")
    );
}
