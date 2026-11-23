use std::rc::Rc;

use super::super::interfaces::{Section, ShellContext};
use crate::features::{bluetooth, display, network};
use crate::shared::icon::Icon;

pub struct SectionDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: Icon,
    pub keywords: &'static [&'static str],
    pub build: fn(&ShellContext) -> Rc<dyn Section>,
}

pub const SECTIONS: &[SectionDescriptor] = &[
    SectionDescriptor {
        id: "display",
        title: "Display",
        icon: Icon::Monitor,
        keywords: &[
            "monitor",
            "screen",
            "resolution",
            "refresh",
            "scale",
            "rotation",
            "orientation",
            "profile",
            "kanshi",
            "brightness",
            "contrast",
            "overdrive",
            "input",
            "hdr",
            "10-bit",
            "vrr",
            "adaptive sync",
            "night light",
            "color temperature",
            "main display",
            "custom refresh",
            "ddc",
        ],
        build: display::build_section,
    },
    SectionDescriptor {
        id: "bluetooth",
        title: "Bluetooth",
        icon: Icon::Bluetooth,
        keywords: &[
            "bluez",
            "pair",
            "pairing",
            "headphones",
            "audio",
            "keyboard",
            "mouse",
            "wireless",
            "connect",
            "device",
            "pan",
            "tethering",
        ],
        build: bluetooth::build_section,
    },
    SectionDescriptor {
        id: "network",
        title: "Network",
        icon: Icon::Globe,
        keywords: &[
            "wifi",
            "wi-fi",
            "wireless",
            "ethernet",
            "internet",
            "vpn",
            "wireguard",
            "openvpn",
            "networkmanager",
            "ip",
            "dns",
            "dhcp",
            "gateway",
            "metered",
            "mac",
            "qr",
            "hotspot",
            "scan",
            "diagnostics",
            "logs",
        ],
        build: network::build_section,
    },
];

/// Sections the sidebar reserves a row for but that are not built yet.
pub struct PlaceholderDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: Icon,
}

pub const PLACEHOLDERS: &[PlaceholderDescriptor] = &[
    PlaceholderDescriptor {
        id: "placeholder-appearance",
        title: "Appearance",
        icon: Icon::Settings,
    },
    PlaceholderDescriptor {
        id: "placeholder-performance",
        title: "Performance",
        icon: Icon::ChartLine,
    },
];
