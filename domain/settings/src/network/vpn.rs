use std::collections::BTreeMap;

/// Who starts and stops a tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpnManager {
    NetworkManager,
    /// A NixOS `wg-quick-*.service` unit; read-only here.
    Systemd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VpnProfile {
    pub id: String,
    pub name: String,
    pub uuid: String,
    pub type_label: String,
    pub active: bool,
    pub device: String,
    pub interface_name: String,
    pub autoconnect: bool,
    pub ipv4_address: String,
    pub ipv4_method: String,
    pub ipv6_address: String,
    pub gateway: String,
    pub dns: Vec<String>,
    pub mtu: Option<u32>,
    pub listen_port: Option<u16>,
    pub peer_count: usize,
    pub peer_public_key: String,
    pub state_label: String,
    pub manager: VpnManager,
    pub systemd_unit: String,
    pub editable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VpnPeerDraft {
    pub public_key: String,
    pub preshared_key: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub persistent_keepalive: String,
}

impl Default for VpnPeerDraft {
    fn default() -> Self {
        Self {
            public_key: String::new(),
            preshared_key: String::new(),
            endpoint: String::new(),
            allowed_ips: "0.0.0.0/0, ::/0".into(),
            persistent_keepalive: "25".into(),
        }
    }
}

impl VpnPeerDraft {
    fn blank() -> Self {
        Self {
            allowed_ips: String::new(),
            persistent_keepalive: String::new(),
            ..Self::default()
        }
    }
}

/// The WireGuard form. `id` and `uuid` are empty for a new tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VpnDraft {
    pub id: String,
    pub uuid: String,
    pub name: String,
    pub interface_name: String,
    pub private_key: String,
    pub addresses: String,
    pub dns: String,
    pub mtu: String,
    pub listen_port: String,
    pub autoconnect: bool,
    pub peers: Vec<VpnPeerDraft>,
}

impl Default for VpnDraft {
    fn default() -> Self {
        Self {
            id: String::new(),
            uuid: String::new(),
            name: String::new(),
            interface_name: String::new(),
            private_key: String::new(),
            addresses: String::new(),
            dns: String::new(),
            mtu: String::new(),
            listen_port: String::new(),
            autoconnect: false,
            peers: vec![VpnPeerDraft::default()],
        }
    }
}

pub const VPN_MIRROR_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpnMirrorPeer {
    pub public_key: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub persistent_keepalive: String,
}

/// The non-secret half of a draft, kept per NetworkManager UUID because `nmcli` cannot
/// read peers back.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpnMirrorEntry {
    pub uuid: String,
    pub name: String,
    pub interface_name: String,
    pub addresses: String,
    pub dns: String,
    pub mtu: String,
    pub listen_port: String,
    pub autoconnect: bool,
    pub peers: Vec<VpnMirrorPeer>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VpnMirrorStore {
    #[serde(default = "mirror_version")]
    pub version: u32,
    #[serde(default)]
    pub profiles: BTreeMap<String, VpnMirrorEntry>,
}

fn mirror_version() -> u32 {
    VPN_MIRROR_VERSION
}

impl Default for VpnMirrorStore {
    fn default() -> Self {
        Self {
            version: VPN_MIRROR_VERSION,
            profiles: BTreeMap::new(),
        }
    }
}

pub fn draft_to_mirror(draft: &VpnDraft) -> Option<VpnMirrorEntry> {
    if draft.uuid.is_empty() {
        return None;
    }
    Some(VpnMirrorEntry {
        uuid: draft.uuid.clone(),
        name: draft.name.clone(),
        interface_name: draft.interface_name.clone(),
        addresses: draft.addresses.clone(),
        dns: draft.dns.clone(),
        mtu: draft.mtu.clone(),
        listen_port: draft.listen_port.clone(),
        autoconnect: draft.autoconnect,
        peers: draft
            .peers
            .iter()
            .map(|p| VpnMirrorPeer {
                public_key: p.public_key.clone(),
                endpoint: p.endpoint.clone(),
                allowed_ips: p.allowed_ips.clone(),
                persistent_keepalive: p.persistent_keepalive.clone(),
            })
            .collect(),
    })
}

pub fn peers_differ(a: &[VpnPeerDraft], b: &[VpnPeerDraft]) -> bool {
    a != b
}

/// `abcdefgh…stuvwxyz`; short keys stay whole.
pub fn truncate_key(key: &str) -> String {
    const KEEP: usize = 8;
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= KEEP * 2 + 1 {
        return key.to_owned();
    }
    let head: String = chars[..KEEP].iter().collect();
    let tail: String = chars[chars.len() - KEEP..].iter().collect();
    format!("{head}…{tail}")
}

/// A wg-quick conf from the form. It carries secrets, so it only ever goes to a private
/// temp file.
pub fn render_wg_conf(draft: &VpnDraft) -> String {
    let mut lines = vec!["[Interface]".to_owned()];
    let mut field = |key: &str, value: &str| {
        let value = value.trim();
        if !value.is_empty() {
            lines.push(format!("{key} = {value}"));
        }
    };
    field("PrivateKey", &draft.private_key);
    field("Address", &draft.addresses);
    field("ListenPort", &draft.listen_port);
    field("DNS", &draft.dns);
    field("MTU", &draft.mtu);

    for peer in draft
        .peers
        .iter()
        .filter(|p| !p.public_key.trim().is_empty())
    {
        lines.push(String::new());
        lines.push("[Peer]".into());
        let mut field = |key: &str, value: &str| {
            let value = value.trim();
            if !value.is_empty() {
                lines.push(format!("{key} = {value}"));
            }
        };
        field("PublicKey", &peer.public_key);
        field("PresharedKey", &peer.preshared_key);
        field("Endpoint", &peer.endpoint);
        field("AllowedIPs", &peer.allowed_ips);
        field("PersistentKeepalive", &peer.persistent_keepalive);
    }
    lines.join("\n") + "\n"
}

/// Reads a wg-quick conf into a draft. `id` and `uuid` stay empty.
pub fn parse_wg_conf(text: &str, name_hint: &str) -> VpnDraft {
    let mut draft = VpnDraft {
        name: name_hint.to_owned(),
        peers: vec![],
        ..VpnDraft::default()
    };
    #[derive(PartialEq)]
    enum Section {
        None,
        Interface,
        Peer,
    }
    let mut section = Section::None;
    let mut peer: Option<VpnPeerDraft> = None;

    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if let Some(done) = peer.take().filter(|p| !p.public_key.trim().is_empty()) {
                draft.peers.push(done);
            }
            match name.to_lowercase().as_str() {
                "interface" => section = Section::Interface,
                "peer" => {
                    section = Section::Peer;
                    peer = Some(VpnPeerDraft::blank());
                }
                _ => section = Section::None,
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim().to_lowercase(), value.trim().to_owned());
        match (&section, key.as_str()) {
            (Section::Interface, "privatekey") => draft.private_key = value,
            (Section::Interface, "address") => draft.addresses = value,
            (Section::Interface, "listenport") => draft.listen_port = value,
            (Section::Interface, "dns") => draft.dns = value,
            (Section::Interface, "mtu") => draft.mtu = value,
            (Section::Peer, "publickey") => {
                if let Some(p) = peer.as_mut() {
                    p.public_key = value;
                }
            }
            (Section::Peer, "presharedkey") => {
                if let Some(p) = peer.as_mut() {
                    p.preshared_key = value;
                }
            }
            (Section::Peer, "endpoint") => {
                if let Some(p) = peer.as_mut() {
                    p.endpoint = value;
                }
            }
            (Section::Peer, "allowedips") => {
                if let Some(p) = peer.as_mut() {
                    p.allowed_ips = value;
                }
            }
            (Section::Peer, "persistentkeepalive") => {
                if let Some(p) = peer.as_mut() {
                    p.persistent_keepalive = value;
                }
            }
            _ => {}
        }
    }
    if let Some(done) = peer.take().filter(|p| !p.public_key.trim().is_empty()) {
        draft.peers.push(done);
    }
    if draft.peers.is_empty() {
        draft.peers.push(VpnPeerDraft::default());
    }
    draft
}

/// What the form is missing, as a sentence for a toast; `None` when it can be saved.
pub fn validate_vpn_draft(draft: &VpnDraft) -> Option<&'static str> {
    if draft.name.trim().is_empty() {
        return Some("Name is required.");
    }
    if draft.private_key.trim().is_empty() {
        return Some("Private key is required.");
    }
    if draft.addresses.trim().is_empty() {
        return Some("Address is required.");
    }
    let peers: Vec<_> = draft
        .peers
        .iter()
        .filter(|p| !p.public_key.trim().is_empty())
        .collect();
    if peers.is_empty() {
        return Some("At least one peer public key is required.");
    }
    if peers.iter().any(|p| p.allowed_ips.trim().is_empty()) {
        return Some("Each peer needs AllowedIPs.");
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpnConfigKind {
    WireGuard,
    OpenVpn,
}

/// `.conf` is WireGuard, `.ovpn` is OpenVPN; anything else is not importable.
pub fn config_kind_for(name: &str) -> Option<VpnConfigKind> {
    let lower = name.to_lowercase();
    if lower.ends_with(".ovpn") {
        Some(VpnConfigKind::OpenVpn)
    } else if lower.ends_with(".conf") {
        Some(VpnConfigKind::WireGuard)
    } else {
        None
    }
}

/// A config file found near the user, offered by the import dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VpnCandidate {
    pub path: String,
    pub name: String,
    pub kind: VpnConfigKind,
    pub mtime_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &str = "\
[Interface]
PrivateKey = priv # secret
Address = 10.0.0.2/32
DNS = 1.1.1.1
MTU = 1380

[Peer]
PublicKey = pub1
Endpoint = 198.51.100.1:51820
AllowedIPs = 0.0.0.0/0, ::/0
PersistentKeepalive = 25

[Peer]
PublicKey = pub2
PresharedKey = psk
AllowedIPs = 10.1.0.0/16
";

    #[test]
    fn parses_interface_and_every_peer() {
        let d = parse_wg_conf(CONF, "wg0");
        assert_eq!(d.name, "wg0");
        assert_eq!(d.private_key, "priv");
        assert_eq!(d.addresses, "10.0.0.2/32");
        assert_eq!(d.mtu, "1380");
        assert_eq!(d.peers.len(), 2);
        assert_eq!(d.peers[0].endpoint, "198.51.100.1:51820");
        assert_eq!(d.peers[1].preshared_key, "psk");
        assert_eq!(d.peers[1].persistent_keepalive, "");
    }

    #[test]
    fn render_then_parse_round_trips() {
        let d = parse_wg_conf(CONF, "wg0");
        let again = parse_wg_conf(&render_wg_conf(&d), "wg0");
        assert_eq!(d, again);
    }

    #[test]
    fn an_empty_conf_still_offers_one_peer_row() {
        assert_eq!(parse_wg_conf("", "x").peers.len(), 1);
    }

    #[test]
    fn peers_without_keys_are_not_rendered() {
        let mut d = parse_wg_conf(CONF, "wg0");
        d.peers.push(VpnPeerDraft::default());
        assert_eq!(render_wg_conf(&d).matches("[Peer]").count(), 2);
    }

    #[test]
    fn validation_names_the_first_gap() {
        let mut d = VpnDraft::default();
        assert_eq!(validate_vpn_draft(&d), Some("Name is required."));
        d.name = "wg".into();
        assert_eq!(validate_vpn_draft(&d), Some("Private key is required."));
        d.private_key = "k".into();
        assert_eq!(validate_vpn_draft(&d), Some("Address is required."));
        d.addresses = "10.0.0.2/32".into();
        assert_eq!(
            validate_vpn_draft(&d),
            Some("At least one peer public key is required.")
        );
        d.peers[0].public_key = "p".into();
        assert_eq!(validate_vpn_draft(&d), None);
        d.peers[0].allowed_ips.clear();
        assert_eq!(validate_vpn_draft(&d), Some("Each peer needs AllowedIPs."));
    }

    #[test]
    fn peer_diff_notices_any_field() {
        let a = parse_wg_conf(CONF, "x").peers;
        let mut b = a.clone();
        assert!(!peers_differ(&a, &b));
        b[0].endpoint.push('1');
        assert!(peers_differ(&a, &b));
    }

    #[test]
    fn mirror_needs_a_uuid_and_keeps_no_secrets() {
        let mut d = parse_wg_conf(CONF, "wg0");
        assert!(draft_to_mirror(&d).is_none());
        d.uuid = "u-1".into();
        let mirror = draft_to_mirror(&d).unwrap();
        let json = serde_json::to_string(&mirror).unwrap();
        assert!(!json.contains("priv") && !json.contains("psk"), "{json}");
        assert!(json.contains("publicKey"), "{json}");
    }

    #[test]
    fn keys_truncate_in_the_middle() {
        assert_eq!(truncate_key("short"), "short");
        let key = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqr=";
        assert_eq!(
            truncate_key(key),
            format!("{}…{}", &key[..8], &key[key.len() - 8..])
        );
    }

    #[test]
    fn config_kinds_follow_the_extension() {
        assert_eq!(config_kind_for("a.CONF"), Some(VpnConfigKind::WireGuard));
        assert_eq!(config_kind_for("a.ovpn"), Some(VpnConfigKind::OpenVpn));
        assert_eq!(config_kind_for("a.txt"), None);
    }
}
