use std::sync::Arc;

use settings_domain::ports::{
    Clock, ConnectionGateway, NetworkEvents, NetworkGateway, NetworkJournal,
    NetworkPreferencesStore, NetworkStats, QrEncoder, SystemdUnits, VpnFiles, VpnGateway,
    VpnMirrorStorePort,
};

#[derive(Clone)]
pub struct NetworkPorts {
    pub gateway: Arc<dyn NetworkGateway>,
    pub connections: Arc<dyn ConnectionGateway>,
    pub vpn: Arc<dyn VpnGateway>,
    pub units: Arc<dyn SystemdUnits>,
    pub events: Arc<dyn NetworkEvents>,
    pub stats: Arc<dyn NetworkStats>,
    pub journal: Arc<dyn NetworkJournal>,
    pub qr: Arc<dyn QrEncoder>,
    pub prefs: Arc<dyn NetworkPreferencesStore>,
    pub mirror: Arc<dyn VpnMirrorStorePort>,
    pub files: Arc<dyn VpnFiles>,
    pub clock: Arc<dyn Clock>,
}
