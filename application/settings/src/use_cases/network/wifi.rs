use settings_domain::network::{
    DeviceKind, NetworkDevice, ScanDecision, ScanOutcome, ScanReason, ScanState,
    access_points_signature, decide_scan, normalize_access_points,
};

use super::NetworkCore;
use crate::error::AppError;
use crate::network_dto::{ScanResult, WifiSavedEntry, WifiView};

fn reason_label(reason: ScanReason) -> &'static str {
    match reason {
        ScanReason::Manual => "manual",
        ScanReason::Open => "open",
        ScanReason::Periodic => "periodic",
        ScanReason::AdapterChange => "adapter-change",
        ScanReason::RadioEnabled => "radio-enabled",
    }
}

/// Keeps the chosen interface while it exists, else prefers a connected one.
pub(super) fn pick_iface(devices: &[NetworkDevice], current: &str) -> String {
    if devices.iter().any(|d| d.iface == current) {
        return current.to_owned();
    }
    devices
        .iter()
        .find(|d| d.connected)
        .or_else(|| devices.first())
        .map(|d| d.iface.clone())
        .unwrap_or_default()
}

impl NetworkCore {
    pub(crate) fn wifi_view(&self) -> Result<WifiView, AppError> {
        let devices = self.ports.gateway.devices(DeviceKind::Wifi)?;
        let radio = self.ports.gateway.wifi_enabled().unwrap_or(true);
        let (selected, scanning) = {
            let mut session = self.session();
            session.wifi_iface = pick_iface(&devices, &session.wifi_iface);
            (session.wifi_iface.clone(), session.scanning)
        };
        let enabled = radio && !selected.is_empty();

        let access_points = if enabled {
            match self.ports.gateway.access_points(&selected) {
                Ok(list) => {
                    let list = normalize_access_points(list);
                    let mut session = self.session();
                    if list.is_empty() {
                        session.cached_aps.clone()
                    } else {
                        session.cached_aps = list.clone();
                        list
                    }
                }
                Err(err) => {
                    self.warn(format!("Wi-Fi list failed: {err}"));
                    self.session().cached_aps.clone()
                }
            }
        } else {
            self.session().cached_aps.clear();
            Vec::new()
        };

        let active_name = devices
            .iter()
            .find(|d| d.iface == selected)
            .filter(|d| d.connected)
            .map(|d| d.connection_name.clone())
            .filter(|name| !name.is_empty());
        let mut saved = self
            .ports
            .connections
            .saved_wifi_profiles()
            .unwrap_or_default()
            .into_iter()
            .map(|profile| WifiSavedEntry {
                active: active_name.as_ref().is_some_and(|name| name == &profile.id),
                id: profile.id,
                ssid: profile.ssid,
            })
            .collect::<Vec<_>>();
        saved.sort_by(|left, right| {
            right
                .active
                .cmp(&left.active)
                .then_with(|| left.ssid.to_lowercase().cmp(&right.ssid.to_lowercase()))
        });

        Ok(WifiView {
            enabled: radio,
            signature: access_points_signature(&access_points),
            devices,
            selected,
            access_points,
            saved,
            scanning,
        })
    }

    pub(crate) fn request_wifi_scan(
        &self,
        reason: ScanReason,
        force: bool,
        page_visible: bool,
    ) -> Result<ScanResult, AppError> {
        let wifi_enabled = self.ports.gateway.wifi_enabled().unwrap_or(false);
        let now = self.ports.clock.now();
        let (decision, iface) = {
            let session = self.session();
            let state = ScanState {
                in_flight: session.scanning,
                wifi_enabled,
                page_visible,
                since_last: session
                    .last_scan
                    .map(|at| now.saturating_duration_since(at)),
            };
            (
                decide_scan(reason, force, &state),
                session.wifi_iface.clone(),
            )
        };
        let tag = reason_label(reason);

        match decision {
            ScanDecision::Skip(skip) => {
                self.warn(format!("Wi-Fi scan skipped ({tag}): {}", skip.label()));
                Ok(ScanResult::Skipped(skip))
            }
            ScanDecision::ListCached(skip) => {
                self.warn(format!("Wi-Fi scan skipped ({tag}): {}", skip.label()));
                Ok(ScanResult::Listed)
            }
            ScanDecision::Scan => {
                self.session().scanning = true;
                let outcome = self.ports.gateway.rescan(&iface);
                {
                    let mut session = self.session();
                    session.scanning = false;
                    session.last_scan = Some(self.ports.clock.now());
                }
                match outcome {
                    ScanOutcome::Ok => {
                        self.info(format!("Wi-Fi scan ok ({tag})"));
                        Ok(ScanResult::Scanned)
                    }
                    ScanOutcome::Rejected(message) => {
                        self.warn(format!("Wi-Fi scan NM-rejected ({tag}): {message}"));
                        Ok(ScanResult::Throttled)
                    }
                    ScanOutcome::Error(message) => {
                        self.error(format!("Wi-Fi scan failed ({tag}): {message}"));
                        Err(AppError::Rejected("Scan failed.".into()))
                    }
                }
            }
        }
    }

    pub(crate) fn set_wifi_enabled(&self, on: bool) -> Result<(), AppError> {
        if let Err(err) = self.ports.gateway.set_wifi_enabled(on) {
            self.error(format!("Wi-Fi radio toggle failed: {err}"));
            return Err(AppError::Rejected(
                "Could not change the Wi-Fi radio.".into(),
            ));
        }
        self.info(if on {
            "WLAN adapter enabled."
        } else {
            "WLAN adapter disabled."
        });
        if on {
            let _ = self.request_wifi_scan(ScanReason::RadioEnabled, false, true);
        } else {
            self.session().cached_aps.clear();
        }
        Ok(())
    }

    pub(crate) fn select_wifi_adapter(&self, iface: String) -> Result<(), AppError> {
        {
            let mut session = self.session();
            session.wifi_iface.clone_from(&iface);
            session.cached_aps.clear();
        }
        self.info(format!("Wi-Fi interface switched to {iface}."));
        Ok(())
    }

    pub(crate) fn connect_wifi(
        &self,
        ssid: String,
        password: Option<String>,
    ) -> Result<(), AppError> {
        let iface = self.session().wifi_iface.clone();
        self.info(format!("Authenticating with {ssid}…"));
        let password = password.filter(|p| !p.is_empty());
        let iface = (!iface.is_empty()).then_some(iface);
        match self
            .ports
            .gateway
            .connect_wifi(&ssid, password.as_deref(), iface.as_deref())
        {
            Ok(()) => {
                self.info(format!("Connected to {ssid}."));
                Ok(())
            }
            Err(err) => {
                self.error(format!("Failed to connect to {ssid}: {err}"));
                Err(AppError::Rejected(format!("Failed to connect to {ssid}.")))
            }
        }
    }

    pub(crate) fn activate_wifi_connection(&self, id: &str) -> Result<(), AppError> {
        let iface = self.session().wifi_iface.clone();
        let label = id.to_owned();
        self.info(format!("Connecting to {label}…"));
        match self
            .ports
            .gateway
            .connection_up(id, (!iface.is_empty()).then_some(iface.as_str()))
        {
            Ok(()) => {
                self.info(format!("Connected to {label}."));
                Ok(())
            }
            Err(err) => {
                self.error(format!("Failed to connect to {label}: {err}"));
                Err(AppError::Rejected(format!("Failed to connect to {label}.")))
            }
        }
    }

    pub(crate) fn disconnect_wifi(&self) -> Result<(), AppError> {
        let devices = self.ports.gateway.devices(DeviceKind::Wifi)?;
        let iface = self.session().wifi_iface.clone();
        let name = devices
            .iter()
            .find(|d| d.iface == iface)
            .map(|d| d.connection_name.clone())
            .filter(|n| !n.is_empty())
            .ok_or_else(|| AppError::Rejected("No active Wi-Fi connection.".into()))?;
        match self.ports.gateway.connection_down(&name) {
            Ok(()) => {
                self.warn("Connection terminated.");
                Ok(())
            }
            Err(err) => {
                self.error(format!("Disconnect failed: {err}"));
                Err(AppError::Rejected("Disconnect failed.".into()))
            }
        }
    }
}
