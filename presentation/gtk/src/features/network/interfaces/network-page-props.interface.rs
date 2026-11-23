use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gtk4 as gtk;
use libadwaita as adw;
use settings_application::network_dto::{
    DiagnosticsView, EthernetView, NetworkOverview, VpnView, WifiProfileView, WifiView,
};
use settings_domain::network::VpnProfile;
pub use settings_application::network_dto::WifiSavedEntry;
use settings_domain::network::{
    IpConfig, IpVersion, NetworkPreferences, QrMatrix, VpnDraft,
};

use super::actions::NetworkActions;

/// What the cards remember between rebuilds.
#[derive(Clone, Default)]
pub struct CardMemory {
    pub expanded: HashSet<String>,
    /// Password text typed per SSID.
    pub drafts: HashMap<String, String>,
}

#[derive(Clone)]
pub struct HomeProps {
    pub overview: NetworkOverview,
    pub actions: NetworkActions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WifiPagination {
    pub saved_page: usize,
    pub saved_page_size: usize,
    pub available_page: usize,
    pub available_page_size: usize,
}

#[derive(Clone)]
pub struct WifiProps {
    pub view: WifiView,
    pub prefs: NetworkPreferences,
    /// A scan the user or the scheduler started is still running.
    pub scanning: bool,
    pub query: String,
    pub pagination: WifiPagination,
    pub memory: CardMemory,
    pub actions: NetworkActions,
}

#[derive(Clone)]
pub struct EthernetProps {
    pub view: EthernetView,
    pub memory: CardMemory,
    pub actions: NetworkActions,
    pub apply_ip: Rc<dyn Fn(IpVersion, IpConfig)>,
}

#[derive(Clone)]
pub struct VpnProps {
    pub view: VpnView,
    pub actions: NetworkActions,
}

#[derive(Clone)]
pub struct VpnDetailProps {
    pub profile: VpnProfile,
    pub actions: NetworkActions,
}

#[derive(Clone)]
pub struct DiagnosticsProps {
    pub view: DiagnosticsView,
    pub actions: NetworkActions,
}

#[derive(Clone)]
pub struct PropertiesProps {
    pub view: WifiProfileView,
    pub actions: NetworkActions,
    pub apply_ip: Rc<dyn Fn(IpVersion, IpConfig)>,
}

#[derive(Clone)]
pub struct ConfirmProps {
    pub heading: String,
    pub body: String,
    pub confirm_label: String,
    pub on_confirm: Rc<dyn Fn()>,
}

#[derive(Clone)]
pub struct QrProps {
    pub ssid: String,
    pub matrix: QrMatrix,
}

#[derive(Clone)]
pub struct VpnFormProps {
    pub draft: VpnDraft,
    pub actions: NetworkActions,
    pub on_save: Rc<dyn Fn(VpnDraft)>,
}

/// The sliding stack of pages.
pub struct PageFrame {
    pub root: gtk::Stack,
    pub home: adw::Bin,
    pub wifi: adw::Bin,
    pub ethernet: adw::Bin,
    pub vpn: adw::Bin,
    pub vpn_detail: adw::Bin,
    pub vpn_form: adw::Bin,
    pub diagnostics: adw::Bin,
    pub properties: adw::Bin,
}

/// The Home page's labels that change every few seconds without a rebuild.
pub struct HomeWidgets {
    pub root: gtk::Widget,
    pub tx: gtk::Label,
    pub rx: gtk::Label,
}
