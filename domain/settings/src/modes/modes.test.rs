use crate::modes::AdvertisedModes;
use crate::value::{Mode, Resolution};

fn modes(list: &[&str]) -> AdvertisedModes {
    list.iter().map(|m| m.parse::<Mode>().unwrap()).collect()
}

fn res(w: u32, h: u32) -> Resolution {
    Resolution::new(w, h)
}

#[test]
fn exact_for_picks_fractional_rate() {
    let m = modes(&["1920x1080@143.97Hz", "1920x1080@60.00Hz"]);
    let hit = m.exact_for(res(1920, 1080), 144).unwrap();
    assert_eq!(hit.kanshi_token(), "1920x1080@143.97");
    assert!(m.exact_for(res(1920, 1080), 120).is_none());
}

#[test]
fn resolutions_sorted_by_pixels() {
    let m = modes(&[
        "1280x720@60Hz",
        "2560x1080@60Hz",
        "1920x1080@60Hz",
        "1920x1080@50Hz",
    ]);
    assert_eq!(
        m.resolutions(),
        vec![res(2560, 1080), res(1920, 1080), res(1280, 720)]
    );
}

#[test]
fn hz_options_include_current_and_advertised_custom_only() {
    let m = modes(&["1920x1080@60.00Hz", "1920x1080@120.00Hz"]);
    assert_eq!(
        m.hz_options(res(1920, 1080), 75, &[120, 144]),
        vec![60, 75, 120]
    );
}

#[test]
fn preferred_rate_falls_back_to_highest() {
    let m = modes(&["1920x1080@60.00Hz", "1920x1080@120.00Hz"]);
    assert_eq!(m.preferred_for_resolution(res(1920, 1080), 60), Some(60));
    assert_eq!(m.preferred_for_resolution(res(1920, 1080), 144), Some(120));
    assert_eq!(m.preferred_for_resolution(res(800, 600), 60), None);
}

#[test]
fn token_for_resolution_keeps_rate_when_possible() {
    let m = modes(&["1680x1050@59.88Hz", "1920x1080@60.00Hz"]);
    assert_eq!(
        m.token_for_resolution(res(1680, 1050), 60)
            .unwrap()
            .kanshi_token(),
        "1680x1050@59.88"
    );
}
