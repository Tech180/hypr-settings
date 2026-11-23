use settings_domain::network::{VpnManager, VpnProfile};

use crate::shared::badge::Tone;

use super::or_dash;

pub fn vpn_source_label(profile: &VpnProfile) -> &'static str {
    match profile.manager {
        VpnManager::Systemd => "System",
        VpnManager::NetworkManager => "NetworkManager",
    }
}

/// Internet reachability while the tunnel is up.
pub fn vpn_connectivity_badge(profile: &VpnProfile) -> Option<(&'static str, Tone)> {
    if !profile.active {
        return None;
    }
    if profile.state_label.contains("Limited") {
        Some(("Limited", Tone::Warning))
    } else if profile.state_label.contains("Full") {
        Some(("Full", Tone::Accent))
    } else {
        None
    }
}

pub fn vpn_row_subtitle(profile: &VpnProfile) -> String {
    format!(
        "{} • {}",
        or_dash(&profile.type_label),
        vpn_source_label(profile)
    )
}

pub fn vpn_subtitle(profile: &VpnProfile) -> String {
    let state = if profile.state_label.is_empty() {
        if profile.active {
            "Connected"
        } else {
            "Disconnected"
        }
    } else {
        profile.state_label.as_str()
    };
    format!("{} • {state}", or_dash(&profile.type_label))
}
