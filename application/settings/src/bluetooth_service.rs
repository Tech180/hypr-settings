use std::sync::Mutex;

use settings_domain::bluetooth::{CodecId, DeviceClass, DeviceId, ProfileKind};
use settings_domain::ports::Subscription;

use crate::api::BluetoothApi;
use crate::bluetooth_ports::BluetoothPorts;
use crate::dto::BluetoothView;
use crate::error::AppError;
use crate::use_cases::bluetooth::refresh_view::refresh_bluetooth_view;

pub struct BluetoothService {
    ports: BluetoothPorts,
    writes: Mutex<()>,
}

impl BluetoothService {
    pub fn new(ports: BluetoothPorts) -> Self {
        let service = Self {
            ports,
            writes: Mutex::new(()),
        };
        if let Ok(prefs) = service.ports.prefs.load() {
            let wanted = prefs.selected_adapter.as_deref();
            let _ = service.ports.gateway.set_active_adapter(wanted);
        }
        service
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.writes.lock().expect("lock")
    }
}

impl BluetoothApi for BluetoothService {
    fn bluetooth_view(&self) -> Result<BluetoothView, AppError> {
        refresh_bluetooth_view(&self.ports)
    }

    fn set_bluetooth_powered(&self, on: bool) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.gateway.set_powered(on)?;
        Ok(())
    }

    fn start_bluetooth_discovery(&self) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.gateway.start_discovery()?;
        Ok(())
    }

    fn stop_bluetooth_discovery(&self) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.gateway.stop_discovery()?;
        Ok(())
    }

    fn select_bluetooth_adapter(&self, address: String) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports
            .gateway
            .set_active_adapter(Some(address.as_str()))?;
        let mut prefs = self.ports.prefs.load()?;
        prefs.selected_adapter = Some(address);
        self.ports.prefs.save(&prefs)?;
        Ok(())
    }

    fn pair_device(&self, id: DeviceId) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.gateway.pair(&id)?;
        Ok(())
    }

    fn connect_device(&self, id: DeviceId) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.gateway.connect(&id)?;
        Ok(())
    }

    fn disconnect_device(&self, id: DeviceId) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.gateway.disconnect(&id)?;
        Ok(())
    }

    fn forget_device(&self, id: DeviceId) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.gateway.remove(&id)?;
        Ok(())
    }

    fn set_device_class(&self, id: DeviceId, class: DeviceClass) -> Result<(), AppError> {
        let _g = self.lock();
        let mut prefs = self.ports.prefs.load()?;
        prefs.set_class(&id, class);
        self.ports.prefs.save(&prefs)?;
        Ok(())
    }

    fn set_device_profile(
        &self,
        id: DeviceId,
        profile: ProfileKind,
        enabled: bool,
    ) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.gateway.set_profile(&id, profile, enabled)?;
        Ok(())
    }

    fn set_device_codec(&self, id: DeviceId, codec: String) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.gateway.set_codec(&id, &CodecId::new(codec))?;
        Ok(())
    }

    fn set_absolute_volume(&self, id: DeviceId, on: bool) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.gateway.set_absolute_volume(&id, on)?;
        Ok(())
    }

    fn set_pan_enabled(&self, on: bool) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.extras.set_pan_enabled(on)?;
        Ok(())
    }

    fn set_avrcp_version(&self, version: String) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.extras.set_avrcp_version(&version)?;
        Ok(())
    }

    fn set_sample_rate_hz(&self, hz: u32) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.extras.set_sample_rate_hz(hz)?;
        Ok(())
    }

    fn set_a2dp_offload_disabled(&self, disabled: bool) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.extras.set_a2dp_offload_disabled(disabled)?;
        Ok(())
    }

    fn set_hci_snoop_enabled(&self, on: bool) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.extras.set_hci_snoop_enabled(on)?;
        Ok(())
    }

    fn clear_gatt_cache(&self) -> Result<(), AppError> {
        let _g = self.lock();
        self.ports.extras.clear_gatt_cache()?;
        Ok(())
    }

    fn subscribe_bluetooth(&self, sink: Box<dyn Fn() + Send>) -> Result<Subscription, AppError> {
        Ok(self.ports.events.subscribe(sink)?)
    }
}
