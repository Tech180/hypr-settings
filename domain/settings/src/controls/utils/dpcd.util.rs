use crate::controls::constants::{DPCD_LINK_RATE_LABELS, DPCD_REV_LABELS};
use crate::controls::interfaces::DpcdCaps;

pub fn parse_dpcd(bytes: &[u8]) -> Option<DpcdCaps> {
    let [rev, max_rate, lanes, ..] = *bytes else {
        return None;
    };
    let max_lanes = lanes & 0x1f;
    (rev != 0 || max_rate != 0 || max_lanes != 0).then_some(DpcdCaps {
        rev,
        max_rate,
        max_lanes,
    })
}

fn label(table: &[(u8, &'static str)], code: u8) -> Option<&'static str> {
    table.iter().find(|(c, _)| *c == code).map(|(_, l)| *l)
}

pub fn dpcd_version(rev: u8) -> Option<&'static str> {
    label(&DPCD_REV_LABELS, rev)
}

/// `HBR3 ×4`; empty when neither the rate nor a sensible lane count is known.
pub fn format_dp_link(caps: DpcdCaps) -> String {
    let rate = label(&DPCD_LINK_RATE_LABELS, caps.max_rate);
    let lanes = (1..=8).contains(&caps.max_lanes).then_some(caps.max_lanes);
    match (rate, lanes) {
        (Some(rate), Some(lanes)) => format!("{rate} ×{lanes}"),
        (Some(rate), None) => rate.to_owned(),
        (None, Some(lanes)) => format!("{lanes}-lane"),
        (None, None) => String::new(),
    }
}
