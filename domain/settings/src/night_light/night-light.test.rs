use super::*;

#[test]
fn kelvin_is_clamped_to_the_slider_range() {
    assert_eq!(Kelvin::clamped(500).get(), KELVIN_MIN);
    assert_eq!(Kelvin::clamped(20_000).get(), KELVIN_MAX);
    assert_eq!(Kelvin::default().get(), KELVIN_DEFAULT);
}

#[test]
fn fractions_round_trip() {
    assert_eq!(Kelvin::from_fraction(0.0).get(), KELVIN_MIN);
    assert_eq!(Kelvin::from_fraction(1.0).get(), KELVIN_MAX);
    let k = Kelvin::clamped(4000);
    assert_eq!(Kelvin::from_fraction(k.fraction()), k);
}
