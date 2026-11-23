use crate::controls::constants::{
    CTA_EXT_HDR_STATIC_METADATA, CTA_EXTENDED_TAG, CTA_VENDOR_BLOCK_TAG, EDID_BLOCK_LEN,
    EDID_CTA_TAG, EDID_HEADER, HDMI_DC_10BIT_MASK, HDMI_FORUM_OUI, HDMI_OUI, HDR_EOTF_HDR_MASK,
};

struct DataBlock<'a> {
    tag: u8,
    extended_tag: Option<u8>,
    payload: &'a [u8],
}

fn has_header(edid: &[u8]) -> bool {
    edid.len() >= EDID_BLOCK_LEN && edid[..8] == EDID_HEADER
}

/// EDID 1.4 bits 4-6 of the video input byte; `None` for analog, undefined or invalid.
fn color_bit_depth(edid: &[u8]) -> Option<u8> {
    if !has_header(edid) {
        return None;
    }
    let input = edid[0x14];
    if input & 0x80 == 0 {
        return None;
    }
    match (input >> 4) & 0x07 {
        1 => Some(6),
        2 => Some(8),
        3 => Some(10),
        4 => Some(12),
        5 => Some(14),
        6 => Some(16),
        _ => None,
    }
}

/// Every CTA-861 data block of every extension block.
fn data_blocks(edid: &[u8]) -> Vec<DataBlock<'_>> {
    let mut blocks = Vec::new();
    if !has_header(edid) {
        return blocks;
    }
    let extensions = usize::from(edid[0x7e]);
    for index in 0..extensions {
        let base = EDID_BLOCK_LEN * (index + 1);
        if edid.len() < base + 4 || edid[base] != EDID_CTA_TAG {
            continue;
        }
        let dtd_start = usize::from(edid[base + 2]);
        let limit = (base + dtd_start.max(4))
            .min(base + EDID_BLOCK_LEN)
            .min(edid.len());
        let mut at = base + 4;
        while at < limit {
            let header = edid[at];
            let tag = header >> 5;
            let len = usize::from(header & 0x1f);
            if len == 0 && tag == 0 {
                break;
            }
            let start = at + 1;
            let end = start + len;
            if end > limit {
                break;
            }
            if tag == CTA_EXTENDED_TAG && len >= 1 {
                blocks.push(DataBlock {
                    tag,
                    extended_tag: Some(edid[start]),
                    payload: &edid[start + 1..end],
                });
            } else {
                blocks.push(DataBlock {
                    tag,
                    extended_tag: None,
                    payload: &edid[start..end],
                });
            }
            at = end;
        }
    }
    blocks
}

/// The vendor specific blocks as (OUI, payload after the OUI).
fn vendor_blocks(edid: &[u8]) -> Vec<(u32, &[u8])> {
    data_blocks(edid)
        .into_iter()
        .filter(|b| b.tag == CTA_VENDOR_BLOCK_TAG && b.payload.len() >= 3)
        .map(|b| {
            let p = b.payload;
            let oui = u32::from(p[0]) | (u32::from(p[1]) << 8) | (u32::from(p[2]) << 16);
            (oui, &p[3..])
        })
        .collect()
}

/// HDR static metadata that offers PQ or HLG.
pub fn edid_supports_hdr(edid: &[u8]) -> bool {
    data_blocks(edid).iter().any(|b| {
        b.extended_tag == Some(CTA_EXT_HDR_STATIC_METADATA)
            && b.payload
                .first()
                .is_some_and(|eotf| eotf & HDR_EOTF_HDR_MASK != 0)
    })
}

/// A colour depth of 10 bits or more in the base block, or HDMI deep colour.
pub fn edid_supports_10bit(edid: &[u8]) -> bool {
    color_bit_depth(edid).is_some_and(|depth| depth >= 10)
        || vendor_blocks(edid).iter().any(|(oui, p)| {
            *oui == HDMI_OUI && p.get(2).is_some_and(|dc| dc & HDMI_DC_10BIT_MASK != 0)
        })
}

/// HDMI version hint: HF-VSDB with a max FRL rate is 2.1, HF-VSDB alone 2.0, a classic
/// HDMI block 1.4.
pub fn hdmi_version(edid: &[u8]) -> Option<&'static str> {
    let mut classic = false;
    let mut forum = false;
    let mut max_frl = 0;
    for (oui, p) in vendor_blocks(edid) {
        if oui == HDMI_OUI {
            classic = true;
        }
        if oui == HDMI_FORUM_OUI {
            forum = true;
            if let Some(byte) = p.get(2) {
                max_frl = max_frl.max(byte >> 4);
            }
            for byte in p.iter().take(8).skip(2) {
                let nibble = byte >> 4;
                if nibble > max_frl && nibble <= 6 {
                    max_frl = nibble;
                }
            }
        }
    }
    match (forum, max_frl, classic) {
        (true, 1.., _) => Some("2.1"),
        (true, 0, _) => Some("2.0"),
        (false, _, true) => Some("1.4"),
        _ => None,
    }
}
