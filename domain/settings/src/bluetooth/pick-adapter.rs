use super::snapshot::AdapterSnapshot;

/// Which adapter in `adapters` to use: `wanted` by address (case-insensitive), else the first.
pub fn pick_adapter(adapters: &[AdapterSnapshot], wanted: Option<&str>) -> Option<usize> {
    if adapters.is_empty() {
        return None;
    }
    if let Some(wanted) = wanted {
        let needle = wanted.to_ascii_lowercase();
        if let Some(index) = adapters
            .iter()
            .position(|a| a.address.to_ascii_lowercase() == needle)
        {
            return Some(index);
        }
    }
    Some(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(address: &str) -> AdapterSnapshot {
        AdapterSnapshot {
            name: "hci0".into(),
            address: address.into(),
            alias: "pc".into(),
            powered: true,
            discoverable: false,
            discovering: false,
            lmp_version: None,
            modalias: None,
        }
    }

    #[test]
    fn empty_list_returns_none() {
        assert_eq!(pick_adapter(&[], Some("aa")), None);
    }

    #[test]
    fn wanted_address_matches_case_insensitive() {
        let adapters = vec![snap("AA:BB:CC:DD:EE:01"), snap("11:22:33:44:55:66")];
        assert_eq!(pick_adapter(&adapters, Some("aa:bb:cc:dd:ee:01")), Some(0));
        assert_eq!(pick_adapter(&adapters, Some("11:22:33:44:55:66")), Some(1));
    }

    #[test]
    fn missing_wanted_falls_back_to_first() {
        let adapters = vec![snap("00:00:00:00:00:01"), snap("00:00:00:00:00:02")];
        assert_eq!(pick_adapter(&adapters, Some("ff:ff:ff:ff:ff:ff")), Some(0));
        assert_eq!(pick_adapter(&adapters, None), Some(0));
    }
}
