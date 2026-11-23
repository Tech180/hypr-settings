use settings_domain::network::{DeviceActionOutcome, DeviceKind};

use super::NetworkCore;
use super::wifi::pick_iface;
use crate::error::AppError;
use crate::network_dto::EthernetView;

const NO_CARRIER: &str = "Ethernet has no carrier — plug in a cable first.";

impl NetworkCore {
    pub(crate) fn ethernet_view(&self) -> Result<EthernetView, AppError> {
        let devices = self.ports.gateway.devices(DeviceKind::Ethernet)?;
        let selected = {
            let mut session = self.session();
            session.eth_iface = pick_iface(&devices, &session.eth_iface);
            session.eth_iface.clone()
        };
        let device = devices.iter().find(|d| d.iface == selected).cloned();
        let profile = device
            .as_ref()
            .map(|d| d.connection_name.clone())
            .filter(|n| !n.is_empty())
            .and_then(|name| self.ports.connections.profile(&name).ok().flatten());
        Ok(EthernetView {
            devices,
            selected,
            device,
            profile,
        })
    }

    pub(crate) fn select_ethernet_adapter(&self, iface: String) -> Result<(), AppError> {
        self.session().eth_iface.clone_from(&iface);
        self.info(format!("Ethernet interface switched to {iface}."));
        Ok(())
    }

    pub(crate) fn set_ethernet_enabled(&self, on: bool) -> Result<(), AppError> {
        let iface = self.session().eth_iface.clone();
        if iface.is_empty() {
            return Err(AppError::Rejected("No Ethernet interface.".into()));
        }
        if on {
            let devices = self.ports.gateway.devices(DeviceKind::Ethernet)?;
            if devices.iter().any(|d| d.iface == iface && !d.enabled) {
                self.warn(NO_CARRIER);
                return Err(AppError::Rejected(NO_CARRIER.into()));
            }
        }
        let outcome = if on {
            self.ports.gateway.device_connect(&iface)
        } else {
            self.ports.gateway.device_disconnect(&iface)
        };
        match outcome {
            DeviceActionOutcome::Ok => {
                if on {
                    self.info("Ethernet interface up.");
                } else {
                    self.warn("Ethernet interface down.");
                }
                Ok(())
            }
            DeviceActionOutcome::NoCarrier(_) => {
                self.warn(NO_CARRIER);
                Err(AppError::Rejected(NO_CARRIER.into()))
            }
            DeviceActionOutcome::Error(message) => {
                self.error(format!("Ethernet toggle failed: {message}"));
                Err(AppError::Rejected(
                    if on {
                        "Failed to bring Ethernet up."
                    } else {
                        "Failed to bring Ethernet down."
                    }
                    .into(),
                ))
            }
        }
    }
}
