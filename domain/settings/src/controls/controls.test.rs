use super::*;

fn ddc(bus: u32, connector: Option<&str>, model: &str, serial: &str) -> DdcDisplay {
    DdcDisplay {
        bus,
        connector: connector.map(str::to_owned),
        model: model.into(),
        serial: serial.into(),
    }
}

#[test]
fn percentages_round_and_clamp() {
    assert_eq!(clamp_percent(-5), 0);
    assert_eq!(clamp_percent(150), 100);
    assert_eq!(percent_from_raw(50, 100), Some(50));
    assert_eq!(percent_from_raw(1, 3), Some(33));
    assert_eq!(percent_from_raw(5, 0), None);
    assert_eq!(raw_from_percent(50, 255), 128);
    assert_eq!(raw_from_percent(100, 100), 100);
    assert_eq!(raw_from_percent(200, 100), 100);
}

#[test]
fn ddc_displays_match_by_connector_then_serial_then_unique_model() {
    let list = [
        ddc(4, Some("DP-3"), "Acme 34", "A1"),
        ddc(5, None, "Other 27", "B2"),
        ddc(6, None, "Twin", "C3"),
        ddc(7, None, "Twin", "C4"),
    ];
    assert_eq!(match_ddc(&list, "dp-3", "", "").unwrap().bus, 4);
    assert_eq!(match_ddc(&list, "HDMI-A-1", "x", "b2").unwrap().bus, 5);
    assert_eq!(
        match_ddc(&list, "HDMI-A-1", "Other 27 Pro", "")
            .unwrap()
            .bus,
        5
    );
    assert!(
        match_ddc(&list, "HDMI-A-1", "Twin", "").is_none(),
        "ambiguous model"
    );
    assert!(match_ddc(&list, "HDMI-A-1", "", "").is_none());
}

#[test]
fn input_labels_know_the_common_codes() {
    assert_eq!(input_label(0x0f), "DP 1");
    assert_eq!(input_label(0x11), "HDMI 1");
    assert_eq!(input_label(0x2a), "Input 0x2a");
}

/// A 256 byte EDID: a digital 10-bit base block and a CTA block carrying the given
/// data blocks.
fn edid_with(blocks: &[&[u8]]) -> Vec<u8> {
    let mut edid = vec![0u8; 256];
    edid[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
    edid[0x14] = 0x80 | (3 << 4);
    edid[0x7e] = 1;
    edid[128] = 0x02;
    let mut at = 132;
    for block in blocks {
        edid[at..at + block.len()].copy_from_slice(block);
        at += block.len();
    }
    edid[130] = u8::try_from(at - 128).unwrap();
    edid
}

#[test]
fn edid_hdr_needs_pq_or_hlg() {
    // extended tag block (7), length 3: tag 0x06, EOTF byte, metadata type
    let pq = edid_with(&[&[0xe3, 0x06, 0b0000_0100, 0x01]]);
    let sdr_only = edid_with(&[&[0xe3, 0x06, 0b0000_0001, 0x01]]);
    assert!(edid_supports_hdr(&pq));
    assert!(!edid_supports_hdr(&sdr_only));
    assert!(!edid_supports_hdr(&[]));
}

#[test]
fn edid_ten_bit_from_base_depth_or_hdmi_deep_colour() {
    assert!(edid_supports_10bit(&edid_with(&[])));
    let mut eight = edid_with(&[]);
    eight[0x14] = 0x80 | (2 << 4);
    assert!(!edid_supports_10bit(&eight));
    // HDMI VSDB: tag 3, len 6, OUI 03 0c 00, A, B, C (deep colour 30 bit)
    let mut hdmi = edid_with(&[&[0x66, 0x03, 0x0c, 0x00, 0x10, 0x00, 0x08]]);
    hdmi[0x14] = 0x80 | (2 << 4);
    assert!(edid_supports_10bit(&hdmi));
}

#[test]
fn hdmi_versions_follow_the_vendor_blocks() {
    let classic = edid_with(&[&[0x65, 0x03, 0x0c, 0x00, 0x10, 0x00]]);
    assert_eq!(hdmi_version(&classic), Some("1.4"));
    let hf = edid_with(&[&[0x66, 0xd8, 0x5d, 0xc4, 0x01, 0x00, 0x00]]);
    assert_eq!(hdmi_version(&hf), Some("2.0"));
    let frl = edid_with(&[&[0x66, 0xd8, 0x5d, 0xc4, 0x01, 0x00, 0x30]]);
    assert_eq!(hdmi_version(&frl), Some("2.1"));
    assert_eq!(hdmi_version(&edid_with(&[])), None);
}

#[test]
fn feature_caps_keep_live_features_switchable() {
    let live_only = FeatureProbe {
        live_hdr: true,
        ..FeatureProbe::default()
    };
    let caps = feature_caps(live_only);
    assert!(caps.hdr.supported && caps.hdr.enabled);
    assert_eq!(caps.hdr.reason, "HDR");
    assert!(!caps.vrr.supported);
    assert_eq!(caps.vrr.reason, "VRR capability unknown");

    let probed = FeatureProbe {
        vrr_capable: Some(false),
        max_bpc: Some(10),
        ..FeatureProbe::default()
    };
    let caps = feature_caps(probed);
    assert_eq!(caps.vrr.reason, "Not VRR capable");
    assert!(caps.ten_bit.supported);
}

#[test]
fn dpcd_and_ports_read_like_the_badge_expects() {
    assert_eq!(parse_dpcd(&[0, 0, 0]), None);
    let caps = parse_dpcd(&[0x14, 0x1e, 0x84]).unwrap();
    assert_eq!(caps.max_lanes, 4);
    assert_eq!(format_dp_link(caps), "HBR3 ×4");
    assert_eq!(dpcd_version(0x14), Some("1.4"));

    let probe = DrmProbe {
        dpcd: Some(caps),
        ..DrmProbe::default()
    };
    let dp = port_info("DP-3", &probe, Some("DP 1".into()));
    assert_eq!(
        (dp.standard.as_str(), dp.version.as_str()),
        ("DisplayPort", "1.4")
    );
    assert_eq!(port_line(&dp), "1.4 · HBR3 ×4");
    assert_eq!(
        port_tooltip(&dp),
        "Host: DisplayPort 1.4, max HBR3 ×4 · Sink input: DP 1"
    );
    assert_eq!(
        port_info("eDP-1", &DrmProbe::default(), None).version,
        "panel"
    );
    assert_eq!(
        port_info("HDMI-A-1", &DrmProbe::default(), None).version,
        "—"
    );
    assert_eq!(port_family("HDMI-A-2"), PortFamily::Hdmi);
}
