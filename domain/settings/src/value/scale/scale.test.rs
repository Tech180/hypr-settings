use crate::value::Resolution;
use crate::value::scale::Scale;

#[test]
fn bounds_are_enforced() {
    assert!(Scale::new(0.49).is_err());
    assert!(Scale::new(0.5).is_ok());
    assert!(Scale::new(4.0).is_ok());
    assert!(Scale::new(4.01).is_err());
    assert!(Scale::new(f64::NAN).is_err());
}

#[test]
fn renders_like_kanshi_expects() {
    assert_eq!(Scale::new(1.0).unwrap().to_string(), "1");
    assert_eq!(Scale::new(1.5).unwrap().to_string(), "1.5");
    assert_eq!(Scale::new(1.25).unwrap().to_string(), "1.25");
    assert_eq!(Scale::new(1.25).unwrap().label(), "1.25×");
}

fn s(v: f64) -> Scale {
    Scale::new(v).unwrap()
}

#[test]
fn validity_follows_integer_logical_sizes() {
    let fhd = Resolution::new(1920, 1080);
    assert!(s(1.5).is_valid_for(fhd));
    assert!(s(1.25).is_valid_for(fhd));
    assert!(s(2.0).is_valid_for(fhd));
    assert!(!s(1.75).is_valid_for(fhd));
    assert!(!s(1.4).is_valid_for(fhd));
    assert!(!s(1.75).is_valid_for(Resolution::new(3840, 2160)));
    assert!(s(1.5).is_valid_for(Resolution::new(2880, 1800)));
}

#[test]
fn nearest_valid_prefers_the_closest_and_always_exists() {
    let fhd = Resolution::new(1920, 1080);
    assert_eq!(s(1.75).nearest_valid_for(fhd), s(1.6));
    assert_eq!(s(1.5).nearest_valid_for(fhd), s(1.5));
    let odd = Resolution::new(1366, 768);
    assert!(s(1.5).nearest_valid_for(odd).is_valid_for(odd));
    assert_eq!(Scale::all_valid_for(odd).first(), Some(&s(0.5)));
}

#[test]
fn offered_scales_hide_invalid_steps_but_keep_the_current_one() {
    let fhd = Resolution::new(1920, 1080);
    let offered = Scale::offered_for(fhd, s(1.0));
    assert!(!offered.contains(&s(1.75)));
    assert!(offered.contains(&s(1.5)) && offered.contains(&s(2.0)));
    assert!(Scale::offered_for(fhd, s(1.33)).contains(&s(1.33)));
}

#[test]
fn clamps_compositor_values() {
    assert_eq!(Scale::clamped(0.1).hundredths(), 50);
    assert_eq!(Scale::clamped(1.6666).to_string(), "1.67");
}
