use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, Options};

use super::used_icons::USED;

#[test]
fn used_icons_parse() {
    let opts = Options::default();
    for &icon in USED {
        usvg::Tree::from_data(icon.svg_str().as_bytes(), &opts).unwrap_or_else(|err| {
            panic!("{icon:?} failed to parse: {err}");
        });
    }
}

#[test]
fn alpha_mask_accepts_unaligned_widths() {
    for side in [10u32, 12, 14, 16, 18, 20, 24, 34, 36] {
        let pixmap = Pixmap::new(side, side).unwrap();
        assert!(
            super::html::alpha_mask(&pixmap, side).is_some(),
            "mask failed at {side}px"
        );
    }
}

#[test]
fn used_icons_render() {
    let opts = Options::default();
    for &icon in USED {
        let tree = usvg::Tree::from_data(icon.svg_str().as_bytes(), &opts).unwrap();
        let mut pixmap = Pixmap::new(48, 48).unwrap();
        let fit = 48.0 / tree.size().height();
        resvg::render(&tree, Transform::from_scale(fit, fit), &mut pixmap.as_mut());
        let covered = pixmap.pixels().iter().filter(|p| p.alpha() > 0).count();
        assert!(
            covered > 40,
            "{icon:?} rendered only {covered} visible pixels"
        );
    }
}
