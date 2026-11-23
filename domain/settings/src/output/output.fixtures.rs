use crate::output::interfaces::{DisplayKind, Output};
use crate::value::{ConnectorName, Mode, Position, RefreshCentiHz, Resolution, Scale, Transform};

pub fn output(name: &str, mode_list: &[&str], current: &str, x: i32) -> Output {
    let connector = ConnectorName::try_from(name).unwrap();
    Output {
        kind: DisplayKind::infer(&connector, ""),
        connector,
        description: String::new(),
        make: String::new(),
        model: String::new(),
        serial: String::new(),
        mode: current.parse().ok(),
        position: Position::new(x, 0),
        scale: Scale::clamped(1.0),
        transform: Transform::Normal,
        enabled: true,
        focused: false,
        modes: mode_list
            .iter()
            .map(|m| m.parse::<Mode>().unwrap())
            .collect(),
        vrr: false,
        hdr: false,
        ten_bit: false,
    }
}

pub fn laptop() -> Output {
    output(
        "eDP-1",
        &["1920x1080@60.00Hz", "1920x1080@47.99Hz"],
        "1920x1080@59.999",
        0,
    )
}

pub fn ultrawide() -> Output {
    output(
        "DP-3",
        &[
            "2560x1080@60.00Hz",
            "2560x1080@100.00Hz",
            "1920x1080@120.00Hz",
            "1920x1080@60.00Hz",
        ],
        "2560x1080@60",
        1920,
    )
}

#[allow(dead_code)]
pub fn res(w: u32, h: u32) -> Resolution {
    Resolution::new(w, h)
}

#[allow(dead_code)]
pub fn hz(v: f64) -> RefreshCentiHz {
    RefreshCentiHz::from_hz(v)
}
