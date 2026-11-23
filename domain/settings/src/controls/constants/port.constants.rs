/// DPCD register offsets on `/dev/drm_dp_aux*`.
pub const DPCD_OFFSET_REV: u64 = 0x00;
pub const DPCD_READ_LEN: usize = 3;

/// DPCD revision byte to a version string.
pub const DPCD_REV_LABELS: [(u8, &str); 8] = [
    (0x10, "1.0"),
    (0x11, "1.1"),
    (0x12, "1.2"),
    (0x13, "1.3"),
    (0x14, "1.4"),
    (0x20, "2.0"),
    (0x22, "2.0"),
    (0x23, "2.1"),
];

/// DPCD MAX_LINK_RATE code to a link rate name.
pub const DPCD_LINK_RATE_LABELS: [(u8, &str); 7] = [
    (0x06, "RBR"),
    (0x0a, "HBR"),
    (0x14, "HBR2"),
    (0x1e, "HBR3"),
    (0x01, "UHBR10"),
    (0x02, "UHBR20"),
    (0x04, "UHBR13.5"),
];

pub const PORT_UNKNOWN_VERSION: &str = "—";

pub const EDID_HEADER: [u8; 8] = [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00];
pub const EDID_BLOCK_LEN: usize = 128;
pub const EDID_CTA_TAG: u8 = 0x02;
pub const CTA_VENDOR_BLOCK_TAG: u8 = 3;
pub const CTA_EXTENDED_TAG: u8 = 7;
pub const CTA_EXT_HDR_STATIC_METADATA: u8 = 0x06;
/// PQ (ST 2084) is bit 2 and HLG bit 3 of the HDR static metadata EOTF byte.
pub const HDR_EOTF_HDR_MASK: u8 = (1 << 2) | (1 << 3);
/// HDMI deep colour flags: 30, 36 and 48 bit.
pub const HDMI_DC_10BIT_MASK: u8 = (1 << 3) | (1 << 4) | (1 << 5);
/// OUIs as the little-endian bytes of the vendor block read as a number.
pub const HDMI_OUI: u32 = 0x000c03;
pub const HDMI_FORUM_OUI: u32 = 0xc4_5dd8;
