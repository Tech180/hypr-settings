#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Wifi,
    Ethernet,
}

/// What the network manager says about reaching the internet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Connectivity {
    Full,
    Limited,
    None,
    #[default]
    Unknown,
}

impl Connectivity {
    pub fn parse(raw: &str) -> Self {
        let lower = raw.trim().to_lowercase();
        if lower.contains("full") {
            Self::Full
        } else if lower.contains("limited") || lower.contains("portal") {
            Self::Limited
        } else if lower.contains("none") {
            Self::None
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    Connected,
    Disconnected,
    /// Cable unplugged, radio off or the interface is not managed.
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDevice {
    pub iface: String,
    pub kind: DeviceKind,
    pub model: String,
    /// False when the interface is unavailable or unmanaged.
    pub enabled: bool,
    pub connected: bool,
    pub speed_mbps: Option<u32>,
    pub mtu: Option<u32>,
    pub rx_bytes: Option<u64>,
    pub tx_bytes: Option<u64>,
    pub mac: String,
    pub ip: String,
    pub gateway: String,
    pub dns: Vec<String>,
    pub ipv6: String,
    pub connection_name: String,
    pub internet_access: bool,
}

impl NetworkDevice {
    pub fn link_state(&self) -> LinkState {
        if self.connected {
            LinkState::Connected
        } else if self.enabled {
            LinkState::Disconnected
        } else {
            LinkState::Unavailable
        }
    }

    /// `iface (model)` for a dropdown row.
    pub fn label(&self) -> String {
        if self.model.is_empty() {
            self.iface.clone()
        } else {
            format!("{} ({})", self.iface, self.model)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(connected: bool, enabled: bool) -> NetworkDevice {
        NetworkDevice {
            iface: "enp3s0".into(),
            kind: DeviceKind::Ethernet,
            model: "Realtek".into(),
            enabled,
            connected,
            speed_mbps: Some(1000),
            mtu: Some(1500),
            rx_bytes: None,
            tx_bytes: None,
            mac: String::new(),
            ip: String::new(),
            gateway: String::new(),
            dns: vec![],
            ipv6: String::new(),
            connection_name: String::new(),
            internet_access: false,
        }
    }

    #[test]
    fn link_state_follows_flags() {
        assert_eq!(device(true, true).link_state(), LinkState::Connected);
        assert_eq!(device(false, true).link_state(), LinkState::Disconnected);
        assert_eq!(device(false, false).link_state(), LinkState::Unavailable);
        assert_eq!(device(true, true).label(), "enp3s0 (Realtek)");
    }

    #[test]
    fn connectivity_parses_nmcli_words() {
        assert_eq!(Connectivity::parse("full\n"), Connectivity::Full);
        assert_eq!(Connectivity::parse("portal"), Connectivity::Limited);
        assert_eq!(Connectivity::parse("none"), Connectivity::None);
        assert_eq!(Connectivity::parse("???"), Connectivity::Unknown);
    }
}
