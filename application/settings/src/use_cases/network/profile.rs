use settings_domain::PortError;
use settings_domain::network::{
    ConnectionProfile, DeviceKind, IpConfig, IpVersion, MacRandomization, QrMatrix,
    validate_ip_config, wifi_qr_payload,
};

use super::NetworkCore;
use crate::error::AppError;
use crate::network_dto::WifiProfileView;

impl NetworkCore {
    /// The saved connection behind the active Wi-Fi link, with live signal details added.
    pub(crate) fn wifi_profile(&self) -> Result<WifiProfileView, AppError> {
        let devices = self.ports.gateway.devices(DeviceKind::Wifi)?;
        let iface = self.session().wifi_iface.clone();
        let device = devices
            .iter()
            .find(|d| d.iface == iface)
            .or_else(|| devices.iter().find(|d| d.connected))
            .cloned();
        let name = device
            .as_ref()
            .map(|d| d.connection_name.clone())
            .filter(|n| !n.is_empty());
        let mut profile = match name {
            Some(name) => self.ports.connections.profile(&name)?,
            None => None,
        };
        if let Some(profile) = profile.as_mut() {
            let cached = self.session().cached_aps.clone();
            if let Some(ap) = cached
                .iter()
                .find(|a| a.connected || a.ssid == profile.ssid)
            {
                if profile.bssid.is_empty() {
                    profile.bssid.clone_from(&ap.bssid);
                }
                profile.frequency_mhz = ap.frequency_mhz;
                profile.strength = ap.strength;
            }
        }
        Ok(WifiProfileView { profile, device })
    }

    pub(crate) fn connection_profile(
        &self,
        id: &str,
    ) -> Result<Option<ConnectionProfile>, AppError> {
        Ok(self.ports.connections.profile(id)?)
    }

    fn change(&self, what: &str, result: Result<(), PortError>) -> Result<(), AppError> {
        match result {
            Ok(()) => {
                self.info(format!("{what} updated."));
                Ok(())
            }
            Err(err) => {
                self.error(format!("{what} failed: {err}"));
                Err(AppError::Rejected(format!("Failed to update {what}.")))
            }
        }
    }

    pub(crate) fn set_autoconnect(&self, id: &str, on: bool) -> Result<(), AppError> {
        let result = self.ports.connections.set_autoconnect(id, on);
        self.change("Auto-Join", result)
    }

    pub(crate) fn set_metered(&self, id: &str, on: bool) -> Result<(), AppError> {
        let result = self.ports.connections.set_metered(id, on);
        self.change("Metered connection", result)
    }

    pub(crate) fn set_mac_randomization(&self, id: &str, random: bool) -> Result<(), AppError> {
        let mode = if random {
            MacRandomization::Random
        } else {
            MacRandomization::Permanent
        };
        let result = self.ports.connections.set_mac_randomization(id, mode);
        self.change("MAC randomization", result)
    }

    pub(crate) fn apply_ip(
        &self,
        id: &str,
        version: IpVersion,
        config: IpConfig,
    ) -> Result<(), AppError> {
        validate_ip_config(version, &config).map_err(|e| AppError::Rejected(e.message))?;
        match self.ports.connections.apply_ip(id, version, &config) {
            Ok(()) => {
                self.info("IP configuration applied.");
                Ok(())
            }
            Err(err) => {
                self.error(format!("IP settings failed: {err}"));
                Err(AppError::Rejected("Failed to apply IP settings.".into()))
            }
        }
    }

    pub(crate) fn forget_connection(&self, id: &str) -> Result<(), AppError> {
        match self.ports.connections.forget(id) {
            Ok(()) => {
                self.warn(format!("Network {id} forgotten."));
                Ok(())
            }
            Err(err) => {
                self.error(format!("Forget failed: {err}"));
                Err(AppError::Rejected("Failed to forget network.".into()))
            }
        }
    }

    pub(crate) fn wifi_qr(&self, id: &str) -> Result<QrMatrix, AppError> {
        let profile = self
            .ports
            .connections
            .profile(id)?
            .ok_or_else(|| AppError::Rejected("No active profile to share.".into()))?;
        let secret = self.ports.connections.wifi_secret(id)?;
        let ssid = if profile.ssid.is_empty() {
            &profile.name
        } else {
            &profile.ssid
        };
        let payload = wifi_qr_payload(ssid, &secret, &profile.security);
        self.ports
            .qr
            .encode(&payload)
            .map_err(|_| AppError::Rejected("QR generation failed.".into()))
    }
}
