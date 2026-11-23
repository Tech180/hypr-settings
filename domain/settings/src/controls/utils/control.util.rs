use crate::controls::interfaces::DdcDisplay;

pub fn clamp_percent(value: i64) -> u8 {
    u8::try_from(value.clamp(0, 100)).unwrap_or(100)
}

/// `current` of `max` as a rounded percentage; `None` when the display reports no range.
pub fn percent_from_raw(current: u32, max: u32) -> Option<u8> {
    (max > 0)
        .then(|| clamp_percent((100 * i64::from(current) + i64::from(max) / 2) / i64::from(max)))
}

pub fn raw_from_percent(percent: u8, max: u32) -> u32 {
    let raw = (u64::from(max) * u64::from(percent.min(100)) + 50) / 100;
    u32::try_from(raw).unwrap_or(max).min(max)
}

/// Finds the DDC display of a Hyprland output: by connector, then serial number, then
/// a model name that only one display matches.
pub fn match_ddc<'a>(
    displays: &'a [DdcDisplay],
    connector: &str,
    model: &str,
    serial: &str,
) -> Option<&'a DdcDisplay> {
    if let Some(found) = displays.iter().find(|d| {
        d.connector
            .as_deref()
            .is_some_and(|c| c.eq_ignore_ascii_case(connector))
    }) {
        return Some(found);
    }
    let serial = serial.trim().to_lowercase();
    if !serial.is_empty()
        && let Some(found) = displays.iter().find(|d| d.serial.to_lowercase() == serial)
    {
        return Some(found);
    }
    let model = model.trim().to_lowercase();
    if model.is_empty() {
        return None;
    }
    let mut matching = displays.iter().filter(|d| {
        let candidate = d.model.to_lowercase();
        !candidate.is_empty() && (candidate.contains(&model) || model.contains(&candidate))
    });
    let first = matching.next()?;
    matching.next().is_none().then_some(first)
}

/// Common MCCS input source values; anything else is shown as hex.
pub fn input_label(code: u8) -> String {
    match code {
        0x01 => "VGA 1".to_owned(),
        0x03 => "DVI 1".to_owned(),
        0x04 => "DVI 2".to_owned(),
        0x0f => "DP 1".to_owned(),
        0x10 => "DP 2".to_owned(),
        0x11 => "HDMI 1".to_owned(),
        0x12 => "HDMI 2".to_owned(),
        other => format!("Input 0x{other:x}"),
    }
}
