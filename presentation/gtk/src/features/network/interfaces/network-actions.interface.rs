use std::rc::Rc;

use settings_domain::network::ScanPolicy;

/// The pages the Network section slides between.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetPage {
    #[default]
    Home,
    Wifi,
    Ethernet,
    Vpn,
    VpnDetail,
    VpnForm,
    Diagnostics,
    Properties,
}

impl NetPage {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Home => "Network & Internet",
            Self::Wifi => "Wi-Fi",
            Self::Ethernet => "Ethernet",
            Self::Vpn => "VPN",
            Self::VpnDetail | Self::VpnForm => "VPN",
            Self::Diagnostics => "Diagnostics",
            Self::Properties => "Connection Properties",
        }
    }

    pub const fn stack_name(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Wifi => "wifi",
            Self::Ethernet => "ethernet",
            Self::Vpn => "vpn",
            Self::VpnDetail => "vpn-detail",
            Self::VpnForm => "vpn-form",
            Self::Diagnostics => "diagnostics",
            Self::Properties => "properties",
        }
    }
}

/// Everything the pages can ask the section to do. Ids are connection names.
#[derive(Clone)]
pub struct NetworkActions {
    pub open: Rc<dyn Fn(NetPage)>,
    pub back: Rc<dyn Fn()>,
    pub open_properties: Rc<dyn Fn()>,

    pub set_wifi_enabled: Rc<dyn Fn(bool)>,
    pub select_wifi_adapter: Rc<dyn Fn(String)>,
    /// A forced scan, from the refresh button.
    pub scan: Rc<dyn Fn()>,
    pub set_scan_policy: Rc<dyn Fn(ScanPolicy, u32)>,
    pub connect_wifi: Rc<dyn Fn(String, Option<String>)>,
    pub activate_wifi: Rc<dyn Fn(String)>,
    pub disconnect_wifi: Rc<dyn Fn()>,
    pub set_query: Rc<dyn Fn(String)>,
    pub set_wifi_saved_page: Rc<dyn Fn(usize)>,
    pub set_wifi_saved_page_size: Rc<dyn Fn(usize)>,
    pub set_wifi_available_page: Rc<dyn Fn(usize)>,
    pub set_wifi_available_page_size: Rc<dyn Fn(usize)>,
    /// A card was opened or closed; remembered across rebuilds.
    pub card_toggled: Rc<dyn Fn(String, bool)>,
    /// What the user typed into a network's password field.
    pub set_draft: Rc<dyn Fn(String, String)>,

    pub set_ethernet_enabled: Rc<dyn Fn(bool)>,
    pub select_ethernet_adapter: Rc<dyn Fn(String)>,

    pub set_autoconnect: Rc<dyn Fn(String, bool)>,
    pub set_metered: Rc<dyn Fn(String, bool)>,
    pub set_mac_random: Rc<dyn Fn(String, bool)>,
    pub forget: Rc<dyn Fn(String)>,
    pub share: Rc<dyn Fn(String)>,

    pub open_vpn: Rc<dyn Fn(String)>,
    pub activate_vpn: Rc<dyn Fn(String)>,
    pub deactivate_vpn: Rc<dyn Fn(String)>,
    pub new_vpn: Rc<dyn Fn()>,
    pub edit_vpn: Rc<dyn Fn(String)>,
    pub delete_vpn: Rc<dyn Fn(String)>,
    pub import_vpn: Rc<dyn Fn()>,

    pub clear_log: Rc<dyn Fn()>,
}
