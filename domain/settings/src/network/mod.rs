//! Network model: access points, devices, IP rules, the scan policy, VPN drafts and the
//! WireGuard conf format, the Wi-Fi share payload and the action log. No IO lives here.

mod connection;
mod device;
mod diagnostics;
mod ip;
mod preferences;
mod qr;
mod scan;
mod vpn;
mod wifi;

pub use connection::{ConnectionProfile, DeviceActionOutcome, MacRandomization, ScanOutcome};
pub use device::{Connectivity, DeviceKind, LinkState, NetworkDevice};
pub use diagnostics::{ActionLog, LogEntry, LogLevel};
pub use ip::{IpConfig, IpError, IpField, IpMethod, IpVersion, prefix_length, validate_ip_config};
pub use preferences::NetworkPreferences;
pub use qr::{QrMatrix, wifi_qr_payload};
pub use scan::{
    SCAN_HARD_FLOOR_MS, SCAN_INTERVALS_MS, ScanDecision, ScanPolicy, ScanReason, ScanSkip,
    ScanState, decide_scan,
};
pub use vpn::{
    VPN_MIRROR_VERSION, VpnCandidate, VpnConfigKind, VpnDraft, VpnManager, VpnMirrorEntry,
    VpnMirrorPeer, VpnMirrorStore, VpnPeerDraft, VpnProfile, config_kind_for, draft_to_mirror,
    parse_wg_conf, peers_differ, render_wg_conf, truncate_key, validate_vpn_draft,
};
pub use wifi::{
    AccessPoint, SavedWifiProfile, access_points_signature, band_label, normalize_access_points,
    security_requires_password,
};
