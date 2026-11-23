//! Maps the layout space (logical pixels, may be negative) onto the arrangement canvas.
//! Same fit as the design: padded, centred, never larger than 0.12 px per logical px.

/// `(x, y, width, height)` in logical pixels.
pub type Rect = (i32, i32, i32, i32);

const PADDING: f64 = 60.0;
const MAX_SCALE: f64 = 0.12;
const EMPTY_SCALE: f64 = 0.08;
const MIN_SCALE: f64 = 0.005;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    /// Canvas pixels per logical pixel.
    pub scale: f64,
    /// Canvas position of the layout origin.
    pub origin: (f64, f64),
}

/// Scale and origin that centre `rects` on the canvas. A given `scale` is kept while the
/// layout still fits at it, so editing the arrangement re-centres the drawing without
/// zooming it in and out.
/// Alias used by the arrangement canvas when re-centring without changing zoom.
pub fn fit_keeping(rects: &[Rect], width: f64, height: f64, scale: Option<f64>) -> Fit {
    fit(rects, width, height, scale)
}

pub fn fit(rects: &[Rect], width: f64, height: f64, scale: Option<f64>) -> Fit {
    if rects.is_empty() || width <= 0.0 || height <= 0.0 {
        return Fit {
            scale: EMPTY_SCALE,
            origin: (width / 2.0, height / 2.0),
        };
    }
    let min_x = rects.iter().map(|r| r.0).min().unwrap_or(0);
    let min_y = rects.iter().map(|r| r.1).min().unwrap_or(0);
    let max_x = rects.iter().map(|r| r.0 + r.2).max().unwrap_or(0);
    let max_y = rects.iter().map(|r| r.1 + r.3).max().unwrap_or(0);
    let total_w = f64::from((max_x - min_x).max(1));
    let total_h = f64::from((max_y - min_y).max(1));

    // A canvas smaller than the padding still gets a sliver of scale rather than a negative one.
    let fitting = ((width - PADDING * 2.0) / total_w)
        .min((height - PADDING * 2.0) / total_h)
        .clamp(MIN_SCALE, MAX_SCALE);
    let scale = scale.filter(|kept| *kept <= fitting).unwrap_or(fitting);
    Fit {
        scale,
        origin: (
            (width - total_w * scale) / 2.0 - f64::from(min_x) * scale,
            (height - total_h * scale) / 2.0 - f64::from(min_y) * scale,
        ),
    }
}

/// Layout position of a canvas point.
pub fn to_logical_point(fit: &Fit, point: (f64, f64)) -> (i32, i32) {
    (
        ((point.0 - fit.origin.0) / fit.scale).round() as i32,
        ((point.1 - fit.origin.1) / fit.scale).round() as i32,
    )
}

/// Index of the first canvas rectangle containing `point`; callers pass the topmost first.
pub fn hit(rects: &[(f64, f64, f64, f64)], point: (f64, f64)) -> Option<usize> {
    rects.iter().position(|&(x, y, w, h)| {
        point.0 >= x && point.0 < x + w && point.1 >= y && point.1 < y + h
    })
}

/// Linear blend between two canvas rectangles, `t` in `0.0..=1.0`.
pub fn lerp(from: (f64, f64, f64, f64), to: (f64, f64, f64, f64), t: f64) -> (f64, f64, f64, f64) {
    let mix = |a: f64, b: f64| a + (b - a) * t;
    (
        mix(from.0, to.0),
        mix(from.1, to.1),
        mix(from.2, to.2),
        mix(from.3, to.3),
    )
}

/// Canvas `(x, y, width, height)` of a layout rectangle.
pub fn to_screen(fit: &Fit, rect: Rect) -> (f64, f64, f64, f64) {
    (
        fit.origin.0 + f64::from(rect.0) * fit.scale,
        fit.origin.1 + f64::from(rect.1) * fit.scale,
        f64::from(rect.2) * fit.scale,
        f64::from(rect.3) * fit.scale,
    )
}

/// Layout-space distance of a canvas-pixel distance.
pub fn to_logical(fit: &Fit, pixels: f64) -> i32 {
    (pixels / fit.scale).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_display_is_capped_and_centred() {
        let f = fit(&[(0, 0, 1920, 1080)], 800.0, 280.0, None);
        assert_eq!(f.scale, MAX_SCALE);
        let (x, y, w, h) = to_screen(&f, (0, 0, 1920, 1080));
        assert!((x + w / 2.0 - 400.0).abs() < 1e-9);
        assert!((y + h / 2.0 - 140.0).abs() < 1e-9);
    }

    #[test]
    fn a_wide_layout_is_limited_by_the_canvas_width() {
        let rects = [(0, 0, 3840, 2160), (3840, 0, 3840, 2160)];
        let f = fit(&rects, 800.0, 280.0, None);
        assert!((f.scale - 160.0 / 2160.0).abs() < 1e-9);
        let left = to_screen(&f, rects[0]);
        let right = to_screen(&f, rects[1]);
        assert!(
            (left.0 + left.2 - right.0).abs() < 1e-9,
            "edges still touch"
        );
    }

    #[test]
    fn negative_positions_stay_inside_the_canvas() {
        let rects = [(-1080, -300, 1080, 1920), (0, 0, 1920, 1080)];
        let f = fit(&rects, 800.0, 280.0, None);
        for r in rects {
            let (x, y, w, h) = to_screen(&f, r);
            assert!(
                x >= 0.0 && y >= 0.0 && x + w <= 800.0 && y + h <= 280.0,
                "{r:?}"
            );
        }
    }

    #[test]
    fn empty_and_unallocated_canvases_do_not_divide_by_zero() {
        assert_eq!(fit(&[], 800.0, 280.0, None).scale, EMPTY_SCALE);
        assert_eq!(fit(&[(0, 0, 10, 10)], 0.0, 0.0, None).scale, EMPTY_SCALE);
    }

    #[test]
    fn pixel_distances_convert_back_to_layout_units() {
        let f = Fit {
            scale: 0.1,
            origin: (0.0, 0.0),
        };
        assert_eq!(to_logical(&f, 30.0), 300);
    }

    #[test]
    fn a_kept_scale_survives_while_the_layout_fits() {
        let side = [(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];
        let first = fit(&side, 800.0, 280.0, None);
        let stacked = [(0, 0, 1920, 1080), (0, 1080, 1920, 1080)];
        let fresh = fit(&stacked, 800.0, 280.0, None);
        let kept = fit(&stacked, 800.0, 280.0, Some(first.scale));
        assert!(
            fresh.scale < first.scale,
            "stacked is taller, so a fresh fit zooms out"
        );
        assert_eq!(fit(&side, 800.0, 280.0, Some(0.05)).scale, 0.05);
        assert_eq!(
            kept.scale, fresh.scale,
            "a scale that no longer fits is dropped"
        );
    }

    #[test]
    fn points_map_back_to_layout_positions() {
        let f = Fit {
            scale: 0.1,
            origin: (50.0, 20.0),
        };
        assert_eq!(to_logical_point(&f, (250.0, 120.0)), (2000, 1000));
    }

    #[test]
    fn hit_testing_prefers_the_first_match() {
        let rects = [(10.0, 10.0, 50.0, 50.0), (0.0, 0.0, 100.0, 100.0)];
        assert_eq!(hit(&rects, (20.0, 20.0)), Some(0));
        assert_eq!(hit(&rects, (80.0, 80.0)), Some(1));
        assert_eq!(hit(&rects, (100.0, 50.0)), None, "right edge is outside");
    }

    #[test]
    fn lerp_blends_every_side() {
        let r = lerp((0.0, 0.0, 10.0, 10.0), (10.0, 20.0, 30.0, 10.0), 0.5);
        assert_eq!(r, (5.0, 10.0, 20.0, 10.0));
    }
}
