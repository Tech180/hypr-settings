//! Drops one known-benign GTK warning so real warnings stay visible.
//!
//! With `GTK_THEME` exported (the session sets `tron-legacy`), libadwaita does not load its
//! stylesheet and the internal `GtkImage` children of every `GtkSwitch` (also inside
//! `AdwSwitchRow`) have no icon source. GTK then logs "reported baselines of minimum
//! -2147483648 ..." on each measure. Nothing is drawn wrong, and the cause is the theme
//! environment, not this app, so the message is filtered instead of worked around in the UI.

use gtk4::glib::{self, LogLevel, LogWriterOutput};

const SWITCH_IMAGE_BASELINE: &str = "reported baselines of minimum -2147483648";

pub fn install() {
    glib::log_set_writer_func(|level, fields| {
        let benign = level == LogLevel::Warning
            && fields.iter().any(|f| {
                f.key() == "MESSAGE" && f.value_str().is_some_and(is_switch_image_baseline)
            });
        if benign {
            LogWriterOutput::Handled
        } else {
            glib::log_writer_default(level, fields)
        }
    });
}

fn is_switch_image_baseline(message: &str) -> bool {
    message.contains("GtkImage") && message.contains(SWITCH_IMAGE_BASELINE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_only_the_gtkimage_baseline_warning() {
        assert!(is_switch_image_baseline(
            "GtkImage 0x1 reported baselines of minimum -2147483648 and natural -2147483648, but sizes of minimum 16"
        ));
        assert!(!is_switch_image_baseline(
            "GtkLabel 0x1 reported baselines of minimum -2147483648"
        ));
        assert!(!is_switch_image_baseline(
            "GtkImage 0x1 reported baselines of minimum 4"
        ));
    }
}
