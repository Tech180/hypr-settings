use super::fixtures::*;
use crate::output::Topology;
use crate::output::interfaces::DisplayKind;
use crate::value::{ConnectorName, Scale, Transform};

#[test]
fn kind_heuristics() {
    let dp = ConnectorName::try_from("DP-1").unwrap();
    let edp = ConnectorName::try_from("eDP-1").unwrap();
    assert_eq!(DisplayKind::infer(&edp, "BOE"), DisplayKind::Laptop);
    assert_eq!(DisplayKind::infer(&dp, "LG OLED TV"), DisplayKind::Tv);
    assert_eq!(DisplayKind::infer(&dp, "Dell P2422H"), DisplayKind::Monitor);
    assert_eq!(
        DisplayKind::infer(&dp, "Hotel Overdrive"),
        DisplayKind::Monitor
    );
}

#[test]
fn logical_size_applies_scale_rounding_and_rotation() {
    let mut o = laptop();
    assert_eq!(o.logical_size(), Some((1920, 1080)));
    o.scale = Scale::new(1.5).unwrap();
    assert_eq!(o.logical_size(), Some((1280, 720)));
    o.transform = Transform::R90;
    assert_eq!(o.logical_size(), Some((720, 1280)));
    o.transform = Transform::Flipped270;
    assert_eq!(o.logical_size(), Some((720, 1280)));
    o.transform = Transform::R180;
    assert_eq!(o.logical_size(), Some((1280, 720)));
    o.scale = Scale::new(1.75).unwrap();
    o.transform = Transform::Normal;
    assert_eq!(o.logical_size(), Some((1097, 617)));
    o.mode = None;
    assert_eq!(o.logical_size(), None);
}

#[test]
fn ultrawide_detection() {
    assert!(ultrawide().is_ultrawide());
    assert!(!laptop().is_ultrawide());
}

#[test]
fn persisted_mode_prefers_advertised_fraction() {
    let mode = laptop().persisted_mode().unwrap();
    assert_eq!(mode.kanshi_token(), "1920x1080@60");
    let mut disabled = laptop();
    disabled.mode = None;
    assert_eq!(disabled.persisted_mode(), None);
}

#[test]
fn bar_host_prefers_the_choice_then_the_laptop() {
    let topology = Topology::new(vec![ultrawide(), laptop()]);
    assert_eq!(
        topology.bar_host(Some("DP-3")).unwrap().connector.as_str(),
        "DP-3"
    );
    assert_eq!(
        topology
            .bar_host(Some("HDMI-A-1"))
            .unwrap()
            .connector
            .as_str(),
        "eDP-1"
    );
    assert_eq!(topology.bar_host(None).unwrap().connector.as_str(), "eDP-1");
    let mut off = ultrawide();
    off.enabled = false;
    let topology = Topology::new(vec![off, laptop()]);
    assert_eq!(
        topology.bar_host(Some("DP-3")).unwrap().connector.as_str(),
        "eDP-1"
    );
    assert!(Topology::default().bar_host(None).is_none());
}
