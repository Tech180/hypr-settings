use super::dpcd::{dpcd_version, format_dp_link};
use super::edid::hdmi_version;
use crate::controls::constants::PORT_UNKNOWN_VERSION;
use crate::controls::interfaces::{DrmProbe, PortFamily, PortInfo};

pub fn port_family(connector: &str) -> PortFamily {
    let name = connector.to_uppercase();
    if ["EDP", "LVDS", "DSI"].iter().any(|p| name.starts_with(p)) {
        PortFamily::Edp
    } else if name.starts_with("DP-") || name.starts_with("DISPLAYPORT") {
        PortFamily::DisplayPort
    } else if name.starts_with("HDMI") {
        PortFamily::Hdmi
    } else if name.starts_with("DVI") {
        PortFamily::Dvi
    } else {
        PortFamily::Other
    }
}

/// Describes the link of a connector from what the kernel exposes. `sink_input` is what
/// the display itself reports through DDC/CI.
pub fn port_info(connector: &str, probe: &DrmProbe, sink_input: Option<String>) -> PortInfo {
    let family = port_family(connector);
    let dpcd = probe.dpcd;
    let link = dpcd
        .filter(|_| matches!(family, PortFamily::DisplayPort | PortFamily::Edp))
        .map(format_dp_link)
        .unwrap_or_default();
    let dp_version = dpcd.and_then(|d| dpcd_version(d.rev));
    let unknown = || PORT_UNKNOWN_VERSION.to_owned();
    let (standard, version) = match family {
        PortFamily::Edp => (
            "eDP".to_owned(),
            dp_version.map_or_else(|| "panel".to_owned(), str::to_owned),
        ),
        PortFamily::DisplayPort => (
            "DisplayPort".to_owned(),
            dp_version.map_or_else(unknown, str::to_owned),
        ),
        PortFamily::Hdmi => (
            "HDMI".to_owned(),
            probe
                .edid
                .as_deref()
                .and_then(hdmi_version)
                .map_or_else(unknown, str::to_owned),
        ),
        PortFamily::Dvi => ("DVI".to_owned(), unknown()),
        PortFamily::Other => (connector.to_owned(), unknown()),
    };
    PortInfo {
        standard,
        version,
        link,
        sink_input,
    }
}

/// One line for the badge: `1.4 · HBR3 ×4`.
pub fn port_line(port: &PortInfo) -> String {
    if port.link.is_empty() {
        port.version.clone()
    } else {
        format!("{} · {}", port.version, port.link)
    }
}

pub fn port_tooltip(port: &PortInfo) -> String {
    let mut host = format!("Host: {}", port.standard);
    if port.version != PORT_UNKNOWN_VERSION {
        host.push(' ');
        host.push_str(&port.version);
    }
    if !port.link.is_empty() {
        host.push_str(&format!(", max {}", port.link));
    }
    match &port.sink_input {
        Some(input) => format!("{host} · Sink input: {input}"),
        None => host,
    }
}
