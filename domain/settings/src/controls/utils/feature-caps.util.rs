use crate::controls::interfaces::{Capability, FeatureCaps, FeatureProbe};

fn capability(
    supported: bool,
    enabled: bool,
    why_supported: &str,
    why_unsupported: &str,
    probe_incomplete: bool,
) -> Capability {
    if enabled && !supported {
        return Capability {
            supported: true,
            enabled,
            reason: if probe_incomplete {
                "Enabled (live); EDID/DRM probe incomplete"
            } else {
                "Enabled (live)"
            }
            .to_owned(),
        };
    }
    Capability {
        supported,
        enabled,
        reason: if supported {
            why_supported
        } else {
            why_unsupported
        }
        .to_owned(),
    }
}

/// Merges the display's own claims with the live compositor state. Whatever is on right
/// now stays switchable so it can always be turned off again.
pub fn feature_caps(probe: FeatureProbe) -> FeatureCaps {
    let bit_from_bpc = probe.max_bpc.is_some_and(|bpc| bpc >= 10);
    let ten_bit = probe.edid_10bit || bit_from_bpc;
    let vrr = probe.vrr_capable == Some(true);
    let missing = probe.vrr_capable.is_none()
        && probe.max_bpc.is_none()
        && !probe.edid_hdr
        && !probe.edid_10bit;
    FeatureCaps {
        hdr: capability(
            probe.edid_hdr || probe.live_hdr,
            probe.live_hdr,
            if probe.edid_hdr {
                "PQ/HLG in EDID"
            } else {
                "HDR"
            },
            "No HDR metadata in EDID",
            missing && probe.live_hdr,
        ),
        ten_bit: capability(
            ten_bit || probe.live_ten_bit,
            probe.live_ten_bit,
            if probe.edid_10bit {
                "Deep color / 10 bpc (EDID)"
            } else if bit_from_bpc {
                "max_bpc ≥ 10 (DRM)"
            } else {
                "10-bit depth"
            },
            "No 10-bit deep color advertised",
            missing && probe.live_ten_bit,
        ),
        vrr: capability(
            vrr || probe.live_vrr,
            probe.live_vrr,
            if probe.vrr_capable == Some(true) {
                "VRR capable (DRM)"
            } else {
                "Adaptive sync (VRR)"
            },
            if probe.vrr_capable == Some(false) {
                "Not VRR capable"
            } else {
                "VRR capability unknown"
            },
            missing && probe.live_vrr,
        ),
    }
}
