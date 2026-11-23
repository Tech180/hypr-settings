use settings_domain::PortError;
use settings_domain::network::{
    AccessPoint, ConnectionProfile, Connectivity, DeviceActionOutcome, DeviceKind, IpConfig,
    IpVersion, MacRandomization, NetworkDevice, SavedWifiProfile, ScanOutcome,
};
use settings_domain::ports::{ConnectionGateway, NetworkGateway};

use crate::connections::{
    ip_modify_args, list_saved_wifi_profiles, mac_mode_value, parse_secret, profile_from_show,
};
use crate::devices::{classify_connect_error, device_from_show, parse_status, type_matches};
use crate::nm::{Nm, port};
use crate::wifi::{AP_FIELDS, classify_scan_error, parse_access_points};

/// Devices, Wi-Fi and saved connections through `nmcli`.
pub struct NmGateway {
    nm: Nm,
}

impl NmGateway {
    pub fn new(nm: Nm) -> Self {
        Self { nm }
    }

    fn ok(result: Result<String, crate::exec::ExecError>) -> Result<(), PortError> {
        result.map(drop).map_err(port)
    }
}

impl NetworkGateway for NmGateway {
    fn devices(&self, kind: DeviceKind) -> Result<Vec<NetworkDevice>, PortError> {
        let status = self
            .nm
            .run(&[
                "-t",
                "-f",
                "DEVICE,TYPE,STATE,CONNECTION",
                "device",
                "status",
            ])
            .map_err(port)?;
        Ok(parse_status(&status)
            .into_iter()
            .filter(|row| type_matches(kind, &row.kind))
            .filter_map(|row| {
                let show = self.nm.device_show(&row.iface)?;
                Some(device_from_show(&row.iface, kind, &show))
            })
            .collect())
    }

    fn wifi_enabled(&self) -> Result<bool, PortError> {
        let out = self.nm.run(&["-t", "-f", "WIFI", "radio"]).map_err(port)?;
        Ok(out.trim().eq_ignore_ascii_case("enabled"))
    }

    fn set_wifi_enabled(&self, on: bool) -> Result<(), PortError> {
        Self::ok(
            self.nm
                .run(&["radio", "wifi", if on { "on" } else { "off" }]),
        )
    }

    fn access_points(&self, iface: &str) -> Result<Vec<AccessPoint>, PortError> {
        let mut args = vec!["-t", "-f", AP_FIELDS, "device", "wifi", "list"];
        if !iface.is_empty() {
            args.extend(["ifname", iface]);
        }
        Ok(parse_access_points(&self.nm.run(&args).map_err(port)?))
    }

    fn rescan(&self, iface: &str) -> ScanOutcome {
        let mut args = vec!["device", "wifi", "rescan"];
        if !iface.is_empty() {
            args.extend(["ifname", iface]);
        }
        match self.nm.run(&args) {
            Ok(_) => ScanOutcome::Ok,
            Err(err) => classify_scan_error(&err.0),
        }
    }

    fn connect_wifi(
        &self,
        ssid: &str,
        password: Option<&str>,
        iface: Option<&str>,
    ) -> Result<(), PortError> {
        let mut args = vec!["device", "wifi", "connect", ssid];
        if let Some(password) = password.filter(|p| !p.is_empty()) {
            args.extend(["password", password]);
        }
        if let Some(iface) = iface.filter(|i| !i.is_empty()) {
            args.extend(["ifname", iface]);
        }
        Self::ok(self.nm.run(&args))
    }

    fn device_connect(&self, iface: &str) -> DeviceActionOutcome {
        match self.nm.run(&["device", "connect", iface]) {
            Ok(_) => DeviceActionOutcome::Ok,
            Err(err) => classify_connect_error(&err.0),
        }
    }

    fn device_disconnect(&self, iface: &str) -> DeviceActionOutcome {
        match self.nm.run(&["device", "disconnect", iface]) {
            Ok(_) => DeviceActionOutcome::Ok,
            Err(err) => DeviceActionOutcome::Error(err.0),
        }
    }

    fn connectivity(&self) -> Connectivity {
        self.nm
            .run(&["-t", "networking", "connectivity"])
            .map_or(Connectivity::Unknown, |out| Connectivity::parse(&out))
    }

    fn connection_up(&self, id: &str, iface: Option<&str>) -> Result<(), PortError> {
        let mut args = vec!["connection", "up", "id", id];
        if let Some(iface) = iface.filter(|name| !name.is_empty()) {
            args.extend(["ifname", iface]);
        }
        Self::ok(self.nm.run(&args))
    }

    fn connection_down(&self, id: &str) -> Result<(), PortError> {
        Self::ok(self.nm.run(&["connection", "down", "id", id]))
    }
}

impl ConnectionGateway for NmGateway {
    fn profile(&self, id: &str) -> Result<Option<ConnectionProfile>, PortError> {
        Ok(self
            .nm
            .connection_show(id)
            .map(|show| profile_from_show(id, &show)))
    }

    fn wifi_secret(&self, id: &str) -> Result<String, PortError> {
        Ok(self
            .nm
            .connection_secrets(id)
            .map(|show| parse_secret(&show))
            .unwrap_or_default())
    }

    fn set_autoconnect(&self, id: &str, on: bool) -> Result<(), PortError> {
        self.nm.modify(
            id,
            &[
                "connection.autoconnect".into(),
                if on { "yes" } else { "no" }.into(),
            ],
        )
    }

    fn set_metered(&self, id: &str, on: bool) -> Result<(), PortError> {
        self.nm.modify(
            id,
            &[
                "connection.metered".into(),
                if on { "yes" } else { "no" }.into(),
            ],
        )
    }

    fn set_mac_randomization(&self, id: &str, mode: MacRandomization) -> Result<(), PortError> {
        self.nm.modify(
            id,
            &[
                "802-11-wireless.cloned-mac-address".into(),
                mac_mode_value(mode).into(),
            ],
        )
    }

    fn apply_ip(&self, id: &str, version: IpVersion, config: &IpConfig) -> Result<(), PortError> {
        self.nm.modify(id, &ip_modify_args(version, config))?;
        Self::ok(self.nm.run(&["connection", "up", "id", id]))
    }

    fn forget(&self, id: &str) -> Result<(), PortError> {
        Self::ok(self.nm.run(&["connection", "delete", "id", id]))
    }

    fn saved_wifi_profiles(&self) -> Result<Vec<SavedWifiProfile>, PortError> {
        list_saved_wifi_profiles(&self.nm)
    }
}
