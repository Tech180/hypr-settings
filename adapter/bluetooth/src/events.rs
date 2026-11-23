use std::sync::{Arc, Mutex};
use std::time::Duration;

use settings_domain::PortError;
use settings_domain::ports::{BluetoothEvents, BluetoothGateway, Subscription};

use crate::gateway::BluerGateway;

/// Notifies when the BlueZ object tree may have changed.
pub struct BluerEvents {
    gateway: Arc<BluerGateway>,
}

impl BluerEvents {
    pub fn new(gateway: Arc<BluerGateway>) -> Self {
        Self { gateway }
    }
}

impl BluetoothEvents for BluerEvents {
    fn subscribe(&self, sink: Box<dyn Fn() + Send>) -> Result<Subscription, PortError> {
        let gateway = Arc::clone(&self.gateway);
        let stop = Arc::new(Mutex::new(false));
        let stop_flag = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !*stop_flag.lock().expect("lock") {
                if gateway.adapter().is_ok() {
                    sink();
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        });
        Ok(Subscription::new(move || {
            *stop.lock().expect("lock") = true;
        }))
    }
}
