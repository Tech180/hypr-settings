use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use super::frame::frame;
use crate::features::network::interfaces::QrProps;

/// Quiet zone around the code, in modules; scanners need it.
const QUIET: usize = 4;
const PIXELS: i32 = 240;

pub fn present(parent: &impl IsA<gtk::Widget>, props: &QrProps) {
    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .halign(gtk::Align::Center)
        .build();
    let matrix = props.matrix.clone();
    let area = gtk::DrawingArea::builder()
        .content_width(PIXELS)
        .content_height(PIXELS)
        .halign(gtk::Align::Center)
        .css_classes(["qr-code"])
        .build();
    area.update_property(&[gtk::accessible::Property::Label(&format!(
        "QR code to join {}",
        props.ssid
    ))]);
    area.set_draw_func(move |_, cr, width, height| {
        let modules = (matrix.size + QUIET * 2) as f64;
        let side = f64::from(width.min(height));
        let unit = side / modules;
        cr.set_source_rgb(1.0, 1.0, 1.0);
        let _ = cr.paint();
        cr.set_source_rgb(0.0, 0.0, 0.0);
        for y in 0..matrix.size {
            for x in 0..matrix.size {
                if matrix.is_dark(x, y) {
                    cr.rectangle(
                        (x + QUIET) as f64 * unit,
                        (y + QUIET) as f64 * unit,
                        unit.ceil(),
                        unit.ceil(),
                    );
                }
            }
        }
        let _ = cr.fill();
    });
    body.append(&area);
    body.append(
        &gtk::Label::builder()
            .label(format!(
                "Scan with a phone camera to join “{}”.",
                props.ssid
            ))
            .wrap(true)
            .justify(gtk::Justification::Center)
            .max_width_chars(36)
            .css_classes(["net-hint"])
            .build(),
    );
    frame("Share Wi-Fi", 380, body.upcast_ref(), None).present(Some(parent));
}
