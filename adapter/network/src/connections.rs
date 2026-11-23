use settings_domain::PortError;
use settings_domain::network::{
    ConnectionProfile, IpConfig, IpMethod, IpVersion, MacRandomization, SavedWifiProfile,
};

use crate::nm::{Nm, port};
use crate::parse::{Show, table};

/// A row of `nmcli -t -f NAME,UUID,TYPE,DEVICE,FILENAME connection show`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionRow {
    pub name: String,
    pub uuid: String,
    pub kind: String,
    pub device: String,
    pub filename: String,
}

pub fn parse_connection_rows(output: &str) -> Vec<ConnectionRow> {
    table(output)
        .into_iter()
        .map(|row| {
            let at = |i: usize| row.get(i).cloned().unwrap_or_default();
            ConnectionRow {
                name: at(0),
                uuid: at(1),
                kind: at(2),
                device: at(3),
                filename: at(4),
            }
        })
        .collect()
}

pub fn parse_secret(show: &Show) -> String {
    show.any(&["802-11-wireless-security.psk", "wifi-sec.psk"])
        .to_owned()
}

fn mac_mode(raw: &str) -> MacRandomization {
    match raw.to_lowercase().as_str() {
        "random" => MacRandomization::Random,
        "stable" => MacRandomization::Stable,
        "permanent" => MacRandomization::Permanent,
        _ => MacRandomization::Default,
    }
}

pub fn mac_mode_value(mode: MacRandomization) -> &'static str {
    match mode {
        MacRandomization::Random => "random",
        MacRandomization::Stable => "stable",
        MacRandomization::Permanent => "permanent",
        MacRandomization::Default => "",
    }
}

fn is_metered(raw: &str) -> bool {
    matches!(raw.to_lowercase().as_str(), "yes" | "true" | "1")
}

pub fn parse_ip_config(show: &Show, version: IpVersion) -> IpConfig {
    let (prefix, runtime) = match version {
        IpVersion::V4 => ("ipv4", "IP4"),
        IpVersion::V6 => ("ipv6", "IP6"),
    };
    let raw = show.first(&format!("{prefix}.method"));
    let method = if raw.eq_ignore_ascii_case("manual") {
        IpMethod::Manual
    } else if raw.eq_ignore_ascii_case("disabled")
        || (version == IpVersion::V6 && raw.eq_ignore_ascii_case("ignore"))
    {
        IpMethod::Disabled
    } else {
        IpMethod::Auto
    };
    let configured = show.first(&format!("{prefix}.addresses"));
    let address_raw = if configured.is_empty() {
        show.prefixed(&format!("{runtime}.ADDRESS"))
            .into_iter()
            .next()
            .unwrap_or_default()
    } else {
        configured.to_owned()
    };
    // `ipv4.addresses` may list several, comma separated; the form edits the first.
    let first = address_raw.split(',').next().unwrap_or("").trim();
    let (address, mask) = first.split_once('/').unwrap_or((first, ""));
    let dns_raw = show.first(&format!("{prefix}.dns"));
    let dns = if dns_raw.is_empty() {
        show.prefixed(&format!("{runtime}.DNS"))
    } else {
        split_list(dns_raw)
    };
    let gateway = show.any(&[&format!("{prefix}.gateway"), &format!("{runtime}.GATEWAY")]);
    IpConfig {
        method,
        address: address.to_owned(),
        prefix_or_mask: mask.to_owned(),
        gateway: gateway.to_owned(),
        dns,
    }
}

/// Splits on commas, semicolons and whitespace.
pub fn split_list(raw: &str) -> Vec<String> {
    raw.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn profile_from_show(id: &str, show: &Show) -> ConnectionProfile {
    let name = show.first("connection.id");
    ConnectionProfile {
        id: id.to_owned(),
        name: if name.is_empty() { id } else { name }.to_owned(),
        ssid: show.first("802-11-wireless.ssid").to_owned(),
        autoconnect: !show
            .first("connection.autoconnect")
            .eq_ignore_ascii_case("no"),
        metered: is_metered(show.first("connection.metered")),
        mac_randomization: mac_mode(show.first("802-11-wireless.cloned-mac-address")),
        ipv4: parse_ip_config(show, IpVersion::V4),
        ipv6: parse_ip_config(show, IpVersion::V6),
        security: match show.first("802-11-wireless-security.key-mgmt") {
            "" => "—",
            other => other,
        }
        .to_owned(),
        bssid: show.first("802-11-wireless.bssid").to_owned(),
        frequency_mhz: 0,
        strength: 0,
        standard: "802.11".to_owned(),
    }
}

pub fn list_saved_wifi_profiles(nm: &Nm) -> Result<Vec<SavedWifiProfile>, PortError> {
    let out = nm
        .run(&[
            "-t",
            "-f",
            "NAME,UUID,TYPE,DEVICE,FILENAME",
            "connection",
            "show",
        ])
        .map_err(port)?;
    Ok(parse_connection_rows(&out)
        .into_iter()
        .filter(|row| row.kind == "802-11-wireless" || row.kind.eq_ignore_ascii_case("wifi"))
        .map(|row| {
            let ssid = nm
                .connection_show(&row.name)
                .map(|show| {
                    let ssid = show.first("802-11-wireless.ssid");
                    if ssid.is_empty() {
                        row.name.clone()
                    } else {
                        ssid.to_owned()
                    }
                })
                .unwrap_or_else(|| row.name.clone());
            SavedWifiProfile { id: row.name, ssid }
        })
        .collect())
}

/// `255.255.255.0` or `24` as the prefix length nmcli wants.
pub fn normalize_prefix(mask_or_prefix: &str, version: IpVersion) -> String {
    settings_domain::network::prefix_length(version, mask_or_prefix).map_or_else(
        || match version {
            IpVersion::V4 => "24".to_owned(),
            IpVersion::V6 => "64".to_owned(),
        },
        |n| n.to_string(),
    )
}

/// The `nmcli connection modify` property/value arguments for an IP editor save.
pub fn ip_modify_args(version: IpVersion, config: &IpConfig) -> Vec<String> {
    let prefix = match version {
        IpVersion::V4 => "ipv4",
        IpVersion::V6 => "ipv6",
    };
    let mut args = Vec::new();
    match config.method {
        IpMethod::Auto => {
            args.extend([format!("{prefix}.method"), "auto".into()]);
            for key in ["addresses", "gateway", "dns"] {
                args.extend([format!("{prefix}.{key}"), String::new()]);
            }
        }
        IpMethod::Disabled => {
            let value = if version == IpVersion::V6 {
                "ignore"
            } else {
                "disabled"
            };
            args.extend([format!("{prefix}.method"), value.into()]);
        }
        IpMethod::Manual => {
            let address = if config.address.contains('/') || config.prefix_or_mask.is_empty() {
                config.address.clone()
            } else {
                format!(
                    "{}/{}",
                    config.address,
                    normalize_prefix(&config.prefix_or_mask, version)
                )
            };
            args.extend([format!("{prefix}.method"), "manual".into()]);
            args.extend([format!("{prefix}.addresses"), address]);
            args.extend([format!("{prefix}.gateway"), config.gateway.clone()]);
            args.extend([format!("{prefix}.dns"), config.dns.join(",")]);
        }
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIFI: &str = include_str!("../tests/fixtures/connection-show-wifi.txt");

    #[test]
    fn a_wifi_profile_is_read() {
        let profile = profile_from_show("Home:5G", &Show::parse(WIFI));
        assert_eq!(profile.name, "Home:5G");
        assert_eq!(profile.ssid, "Home:5G");
        assert!(profile.autoconnect);
        assert!(profile.metered);
        assert_eq!(profile.mac_randomization, MacRandomization::Stable);
        assert_eq!(profile.security, "wpa-psk");
        assert_eq!(profile.ipv4.method, IpMethod::Manual);
        assert_eq!(profile.ipv4.address, "192.168.1.50");
        assert_eq!(profile.ipv4.prefix_or_mask, "24");
        assert_eq!(profile.ipv4.gateway, "192.168.1.1");
        assert_eq!(profile.ipv4.dns, ["1.1.1.1", "9.9.9.9"]);
        assert_eq!(profile.ipv6.method, IpMethod::Auto);
        assert!(profile.ipv6.address.is_empty());
    }

    #[test]
    fn the_secret_comes_from_either_key() {
        let show = Show::parse("802-11-wireless-security.psk:pa\\:ss\n");
        assert_eq!(parse_secret(&show), "pa:ss");
        assert_eq!(parse_secret(&Show::parse("wifi-sec.psk:x\n")), "x");
        assert_eq!(parse_secret(&Show::default()), "");
    }

    #[test]
    fn rows_of_connection_show() {
        let rows = parse_connection_rows(
            "Home\\:5G:uuid-1:802-11-wireless:wlan0:/etc/NetworkManager/system-connections/Home.nmconnection\nmullvad:uuid-2:wireguard::/run/NetworkManager/system-connections/mullvad.nmconnection\n",
        );
        assert_eq!(rows[0].name, "Home:5G");
        assert_eq!(rows[0].device, "wlan0");
        assert_eq!(rows[1].kind, "wireguard");
        assert_eq!(rows[1].device, "");
    }

    #[test]
    fn manual_addresses_get_a_prefix_from_a_mask() {
        let config = IpConfig {
            method: IpMethod::Manual,
            address: "10.0.0.5".into(),
            prefix_or_mask: "255.255.0.0".into(),
            gateway: "10.0.0.1".into(),
            dns: vec!["1.1.1.1".into(), "8.8.8.8".into()],
        };
        assert_eq!(
            ip_modify_args(IpVersion::V4, &config),
            [
                "ipv4.method",
                "manual",
                "ipv4.addresses",
                "10.0.0.5/16",
                "ipv4.gateway",
                "10.0.0.1",
                "ipv4.dns",
                "1.1.1.1,8.8.8.8"
            ]
        );
    }

    #[test]
    fn disabled_uses_the_stack_specific_method() {
        let v4 = ip_modify_args(IpVersion::V4, &IpConfig {
            method: IpMethod::Disabled,
            ..IpConfig::default()
        });
        assert_eq!(v4, ["ipv4.method", "disabled"]);
        let v6 = ip_modify_args(IpVersion::V6, &IpConfig {
            method: IpMethod::Disabled,
            ..IpConfig::default()
        });
        assert_eq!(v6, ["ipv6.method", "ignore"]);
    }

    #[test]
    fn automatic_clears_the_manual_fields() {
        let args = ip_modify_args(IpVersion::V6, &IpConfig::default());
        assert_eq!(
            args,
            [
                "ipv6.method",
                "auto",
                "ipv6.addresses",
                "",
                "ipv6.gateway",
                "",
                "ipv6.dns",
                ""
            ]
        );
    }

    #[test]
    fn prefixes_normalize() {
        assert_eq!(normalize_prefix("24", IpVersion::V4), "24");
        assert_eq!(normalize_prefix("255.255.255.0", IpVersion::V4), "24");
        assert_eq!(normalize_prefix("", IpVersion::V6), "64");
    }
}
