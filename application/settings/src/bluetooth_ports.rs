use std::sync::Arc;

use settings_domain::ports::{
    BluetoothEvents, BluetoothExtras, BluetoothGateway, BluetoothPreferencesStore,
};

#[derive(Clone)]
pub struct BluetoothPorts {
    pub gateway: Arc<dyn BluetoothGateway>,
    pub events: Arc<dyn BluetoothEvents>,
    pub extras: Arc<dyn BluetoothExtras>,
    pub prefs: Arc<dyn BluetoothPreferencesStore>,
}
