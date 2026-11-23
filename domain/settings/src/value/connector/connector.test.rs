use crate::value::connector::ConnectorName;

#[test]
fn accepts_real_connectors() {
    for name in ["eDP-1", "DP-3", "HDMI-A-1", "Unknown-1"] {
        assert!(ConnectorName::try_from(name).is_ok(), "{name}");
    }
}

#[test]
fn rejects_unsafe_names() {
    for name in ["", "DP 3", "DP-3\"; os.exit()", "a/b"] {
        assert!(ConnectorName::try_from(name).is_err(), "{name:?}");
    }
}

#[test]
fn internal_prefixes() {
    assert!(ConnectorName::try_from("eDP-1").unwrap().is_internal());
    assert!(ConnectorName::try_from("LVDS-1").unwrap().is_internal());
    assert!(!ConnectorName::try_from("DP-3").unwrap().is_internal());
}
