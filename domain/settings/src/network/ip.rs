use std::net::{Ipv4Addr, Ipv6Addr};

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpVersion {
    V4,
    V6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IpMethod {
    #[default]
    Auto,
    Manual,
    /// The stack is off (`ipv4.method=disabled`, `ipv6.method=ignore`).
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IpConfig {
    pub method: IpMethod,
    pub address: String,
    /// A prefix length (`24`) or, for IPv4, a netmask (`255.255.255.0`).
    pub prefix_or_mask: String,
    pub gateway: String,
    pub dns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpField {
    Address,
    Prefix,
    Gateway,
    Dns,
}

/// Which field is wrong and why, so the form can mark it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct IpError {
    pub field: IpField,
    pub message: String,
}

fn parses(version: IpVersion, text: &str) -> bool {
    match version {
        IpVersion::V4 => text.parse::<Ipv4Addr>().is_ok(),
        IpVersion::V6 => text.parse::<Ipv6Addr>().is_ok(),
    }
}

/// The prefix length of `/24` or `255.255.255.0`; `None` when it is neither.
pub fn prefix_length(version: IpVersion, text: &str) -> Option<u8> {
    let text = text.trim();
    let max = match version {
        IpVersion::V4 => 32,
        IpVersion::V6 => 128,
    };
    if let Ok(n) = text.parse::<u8>() {
        return (n <= max).then_some(n);
    }
    if version == IpVersion::V4 {
        let mask = u32::from(text.parse::<Ipv4Addr>().ok()?);
        let ones = mask.leading_ones();
        if mask == u32::MAX.checked_shl(32 - ones).unwrap_or(0) {
            return u8::try_from(ones).ok();
        }
    }
    None
}

/// Checks a manual configuration before anything reaches NetworkManager. Automatic
/// configuration has nothing to check.
pub fn validate_ip_config(version: IpVersion, config: &IpConfig) -> Result<(), IpError> {
    if matches!(config.method, IpMethod::Auto | IpMethod::Disabled) {
        return Ok(());
    }
    let fail = |field, message: &str| {
        Err(IpError {
            field,
            message: message.into(),
        })
    };
    let address = config.address.split('/').next().unwrap_or("").trim();
    if !parses(version, address) {
        return fail(IpField::Address, "Enter a valid IP address.");
    }
    if !config.address.contains('/') && prefix_length(version, &config.prefix_or_mask).is_none() {
        return fail(IpField::Prefix, "Enter a prefix length or a netmask.");
    }
    if !config.gateway.trim().is_empty() && !parses(version, config.gateway.trim()) {
        return fail(IpField::Gateway, "Enter a valid gateway address.");
    }
    if config.dns.iter().any(|d| !parses(version, d.trim())) {
        return fail(IpField::Dns, "Every DNS server must be an IP address.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manual(address: &str, prefix: &str, gateway: &str, dns: &[&str]) -> IpConfig {
        IpConfig {
            method: IpMethod::Manual,
            address: address.into(),
            prefix_or_mask: prefix.into(),
            gateway: gateway.into(),
            dns: dns.iter().map(|d| (*d).into()).collect(),
        }
    }

    #[test]
    fn automatic_needs_no_fields() {
        assert!(validate_ip_config(IpVersion::V4, &IpConfig::default()).is_ok());
    }

    #[test]
    fn manual_checks_each_field() {
        let ok = manual("192.168.1.5", "24", "192.168.1.1", &["1.1.1.1"]);
        assert!(validate_ip_config(IpVersion::V4, &ok).is_ok());

        let err = validate_ip_config(IpVersion::V4, &manual("300.1.1.1", "24", "", &[]));
        assert_eq!(err.unwrap_err().field, IpField::Address);
        let err = validate_ip_config(IpVersion::V4, &manual("10.0.0.2", "99", "", &[]));
        assert_eq!(err.unwrap_err().field, IpField::Prefix);
        let err = validate_ip_config(IpVersion::V4, &manual("10.0.0.2", "24", "nope", &[]));
        assert_eq!(err.unwrap_err().field, IpField::Gateway);
        let err = validate_ip_config(IpVersion::V4, &manual("10.0.0.2", "24", "", &["x"]));
        assert_eq!(err.unwrap_err().field, IpField::Dns);
    }

    #[test]
    fn cidr_in_the_address_replaces_the_prefix() {
        let cfg = manual("10.0.0.2/8", "", "", &[]);
        assert!(validate_ip_config(IpVersion::V4, &cfg).is_ok());
    }

    #[test]
    fn netmasks_become_prefix_lengths() {
        assert_eq!(prefix_length(IpVersion::V4, "255.255.255.0"), Some(24));
        assert_eq!(prefix_length(IpVersion::V4, "255.0.0.0"), Some(8));
        assert_eq!(prefix_length(IpVersion::V4, "255.0.255.0"), None);
        assert_eq!(prefix_length(IpVersion::V6, "64"), Some(64));
        assert_eq!(prefix_length(IpVersion::V6, "129"), None);
    }

    #[test]
    fn ipv6_is_checked_as_ipv6() {
        let cfg = manual("fd00::2", "64", "fd00::1", &["2606:4700:4700::1111"]);
        assert!(validate_ip_config(IpVersion::V6, &cfg).is_ok());
        assert!(validate_ip_config(IpVersion::V4, &cfg).is_err());
    }
}
