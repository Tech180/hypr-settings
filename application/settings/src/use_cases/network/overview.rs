use std::time::Duration;

use settings_domain::network::{DeviceKind, NetworkDevice};

use super::{NetworkCore, RateSample};
use crate::error::AppError;
use crate::network_dto::{NetworkOverview, NetworkRates, PrimaryLink};

/// Rates are only worth reporting over a window this long.
const MIN_SAMPLE_WINDOW: Duration = Duration::from_millis(400);

fn first_connected(devices: &[NetworkDevice]) -> Option<&NetworkDevice> {
    devices.iter().find(|d| d.connected)
}

impl NetworkCore {
    pub(crate) fn network_overview(&self) -> Result<NetworkOverview, AppError> {
        let wifi = self
            .ports
            .gateway
            .devices(DeviceKind::Wifi)
            .unwrap_or_default();
        let wired = self
            .ports
            .gateway
            .devices(DeviceKind::Ethernet)
            .unwrap_or_default();
        let vpn = self.ports.vpn.profiles().unwrap_or_default();
        let wifi_enabled = self.ports.gateway.wifi_enabled().unwrap_or(false);
        let connectivity = self.ports.gateway.connectivity();

        let wired_link = first_connected(&wired);
        let wifi_link = first_connected(&wifi);
        let (primary, title, iface) = match (wired_link, wifi_link) {
            (Some(d), _) => (PrimaryLink::Wired, d.iface.clone(), d.iface.clone()),
            (None, Some(d)) => {
                let name = if d.connection_name.is_empty() {
                    d.iface.clone()
                } else {
                    d.connection_name.clone()
                };
                (PrimaryLink::Wifi, name, d.iface.clone())
            }
            (None, None) => (
                PrimaryLink::Offline,
                "Not connected".to_owned(),
                String::new(),
            ),
        };
        self.session().primary_iface = iface;

        Ok(NetworkOverview {
            primary,
            title,
            connectivity,
            wifi_present: !wifi.is_empty(),
            wifi_enabled,
            wifi_ssid: wifi_link
                .map(|d| d.connection_name.clone())
                .filter(|n| !n.is_empty()),
            ethernet_present: !wired.is_empty(),
            ethernet_connected: wired_link.is_some(),
            vpn_total: vpn.len(),
            vpn_active: vpn.iter().filter(|p| p.active).count(),
            has_wifi_profile: wifi_link.is_some_and(|d| !d.connection_name.is_empty()),
        })
    }

    /// Cheap enough to poll: reads two counters and compares them with the last reading.
    pub(crate) fn network_rates(&self) -> NetworkRates {
        let iface = self.session().primary_iface.clone();
        if iface.is_empty() {
            self.session().sample = None;
            return NetworkRates::default();
        }
        let Some((rx, tx)) = self.ports.stats.counters(&iface) else {
            return NetworkRates::default();
        };
        let now = self.ports.clock.now();
        let mut session = self.session();
        let rates = match &session.sample {
            Some(prev) if prev.iface == iface => {
                let window = now.saturating_duration_since(prev.at);
                if window >= MIN_SAMPLE_WINDOW {
                    let secs = window.as_secs_f64();
                    let per_sec = |now: u64, before: u64| {
                        // A counter that went back means the interface was reset.
                        Some((now.saturating_sub(before) as f64 / secs) as u64)
                    };
                    NetworkRates {
                        rx_per_sec: per_sec(rx, prev.rx),
                        tx_per_sec: per_sec(tx, prev.tx),
                    }
                } else {
                    return NetworkRates::default();
                }
            }
            _ => NetworkRates::default(),
        };
        session.sample = Some(RateSample {
            at: now,
            iface,
            rx,
            tx,
        });
        rates
    }
}
