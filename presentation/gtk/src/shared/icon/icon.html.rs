use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use gtk::cairo;
use gtk::prelude::*;
use gtk4 as gtk;
use lucide_static_svg::Icon;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, Options};

use super::props::IconProps;

static TREES: OnceLock<Mutex<HashMap<Icon, usvg::Tree>>> = OnceLock::new();

pub fn build(props: &IconProps) -> gtk::Widget {
    let icon = props.icon;
    let logical = props.size.max(1);
    let area = gtk::DrawingArea::builder()
        .content_width(logical)
        .content_height(logical)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .css_classes(["icon"])
        .build();
    area.set_draw_func(move |area, cr, width, height| {
        let Some(tree) = cached_tree(icon) else {
            return;
        };
        let scale = f64::from(area.scale_factor().max(1));
        let side = (f64::from(width.min(height)) * scale).round().max(1.0) as u32;
        let Some(mut pixmap) = Pixmap::new(side, side) else {
            return;
        };
        let fit = side as f32 / tree.size().height();
        resvg::render(&tree, Transform::from_scale(fit, fit), &mut pixmap.as_mut());

        let Some(mask) = alpha_mask(&pixmap, side) else {
            return;
        };
        let color = glyph_color(area);
        cr.set_source_rgba(
            f64::from(color.red()),
            f64::from(color.green()),
            f64::from(color.blue()),
            f64::from(color.alpha()),
        );
        let offset_x = (f64::from(width) - f64::from(side) / scale) / 2.0;
        let offset_y = (f64::from(height) - f64::from(side) / scale) / 2.0;
        let _ = cr.save();
        cr.scale(1.0 / scale, 1.0 / scale);
        let _ = cr.mask_surface(&mask, offset_x * scale, offset_y * scale);
        let _ = cr.restore();
    });
    area.upcast()
}

fn trees() -> &'static Mutex<HashMap<Icon, usvg::Tree>> {
    TREES.get_or_init(|| {
        let mut map = HashMap::new();
        for &icon in super::used_icons::USED {
            if let Some(tree) = parse_tree(icon) {
                map.insert(icon, tree);
            } else {
                tracing::warn!("icon {icon:?} failed to parse");
            }
        }
        Mutex::new(map)
    })
}

fn cached_tree(icon: Icon) -> Option<usvg::Tree> {
    let cache = trees().lock().ok()?;
    if let Some(tree) = cache.get(&icon) {
        return Some(tree.clone());
    }
    drop(cache);
    let tree = parse_tree(icon)?;
    if let Ok(mut cache) = trees().lock() {
        cache.insert(icon, tree.clone());
    }
    Some(tree)
}

/// `DrawingArea::color()` is often unset when CSS only says `inherit`; walk ancestors.
fn glyph_color(area: &gtk::DrawingArea) -> gtk::gdk::RGBA {
    let mut widget = Some(area.clone().upcast::<gtk::Widget>());
    while let Some(w) = widget {
        let color = w.color();
        if color.alpha() > 0.0 {
            return color;
        }
        widget = w.parent();
    }
    gtk::gdk::RGBA::new(1.0, 1.0, 1.0, 1.0)
}

fn parse_tree(icon: Icon) -> Option<usvg::Tree> {
    usvg::Tree::from_data(icon.svg_str().as_bytes(), &Options::default()).ok()
}

/// Cairo requires A8 rows padded to a 4-byte stride; an unpadded width (10, 14, 18, 34 px...)
/// makes `create_for_data` fail and the icon vanish.
pub(super) fn alpha_mask(pixmap: &Pixmap, side: u32) -> Option<cairo::ImageSurface> {
    let size = i32::try_from(side).ok()?;
    let stride = cairo::Format::A8.stride_for_width(side).ok()?;
    let row = side as usize;
    let stride_len = usize::try_from(stride).ok()?;
    let mut data = vec![0u8; stride_len * row];
    for (y, line) in pixmap.pixels().chunks(row).enumerate() {
        for (x, px) in line.iter().enumerate() {
            data[y * stride_len + x] = px.alpha();
        }
    }
    cairo::ImageSurface::create_for_data(data, cairo::Format::A8, size, size, stride).ok()
}
