use crate::value::RefreshCentiHz;
use crate::value::mode::Mode;

#[test]
fn parses_hyprland_and_kanshi_forms() {
    let a: Mode = "1920x1080@143.97Hz".parse().unwrap();
    assert_eq!(a.refresh, RefreshCentiHz::from_hz(143.97));
    let b: Mode = "1920x1080@60".parse().unwrap();
    assert_eq!(b.kanshi_token(), "1920x1080@60");
    assert_eq!(b.hypr_token(), "1920x1080@60.00");
}

#[test]
fn rejects_garbage() {
    assert!("1920x1080".parse::<Mode>().is_err());
    assert!("axb@60".parse::<Mode>().is_err());
    assert!("0x0@60".parse::<Mode>().is_err());
}

#[test]
fn fractional_token_keeps_two_decimals() {
    let m: Mode = "2560x1080@164.99Hz".parse().unwrap();
    assert_eq!(m.kanshi_token(), "2560x1080@164.99");
}
