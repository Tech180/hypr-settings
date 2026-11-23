use crate::value::refresh::RefreshCentiHz;

#[test]
fn rounds_to_whole_hertz() {
    assert_eq!(RefreshCentiHz::from_hz(143.97).rounded_hz(), 144);
    assert_eq!(RefreshCentiHz::from_hz(59.999).rounded_hz(), 60);
    assert_eq!(RefreshCentiHz::from_hz(47.99).rounded_hz(), 48);
}

#[test]
fn renders_kanshi_fragment() {
    assert_eq!(RefreshCentiHz::from_hz(60.0).to_string(), "60");
    assert_eq!(RefreshCentiHz::from_hz(164.99).to_string(), "164.99");
    assert_eq!(RefreshCentiHz::from_hz(59.94).to_string(), "59.94");
}

#[test]
fn parses_decimal_text() {
    assert_eq!(
        RefreshCentiHz::parse("143.97"),
        Some(RefreshCentiHz::from_hz(143.97))
    );
    assert_eq!(
        RefreshCentiHz::parse("60"),
        Some(RefreshCentiHz::from_hz(60.0))
    );
    assert_eq!(RefreshCentiHz::parse("x"), None);
}
