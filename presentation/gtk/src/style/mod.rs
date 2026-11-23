//! Merges every component stylesheet into a single provider loaded once per display.

use gtk4 as gtk;

const FALLBACK_TOKENS: &str = include_str!("tokens.css");

const SHEETS: &[&str] = &[
    include_str!("../shared/icon/icon.css"),
    include_str!("../shared/section-header/section-header.css"),
    include_str!("../shared/segmented/segmented.css"),
    include_str!("../shared/badge/badge.css"),
    include_str!("../shared/button/button.css"),
    include_str!("../shared/dropdown/dropdown.css"),
    include_str!("../shared/select/select.css"),
    include_str!("../shared/pagination/pagination.css"),
    include_str!("../shared/icon-tile/icon-tile.css"),
    include_str!("../shared/search-field/search-field.css"),
    include_str!("../shared/settings-group/settings-group.css"),
    include_str!("../shared/slider/slider.css"),
    include_str!("../shared/tab-bar/tab-bar.css"),
    include_str!("../shared/toggle/toggle.css"),
    include_str!("../features/settings-shell/shell.css"),
    include_str!("../features/settings-shell/components/sidebar/sidebar.css"),
    include_str!("../features/settings-shell/components/section-search/section-search.css"),
    include_str!("../features/bluetooth/page.css"),
    include_str!("../features/network/page.css"),
    include_str!("../features/display/page.css"),
    include_str!("../features/display/components/arrangement-canvas/arrangement-canvas.css"),
    include_str!("../features/display/components/display-dashboard/display-dashboard.css"),
    include_str!("../features/display/components/hz-group/hz-group.css"),
    include_str!("../features/display/components/feature-grid/feature-grid.css"),
    include_str!("../features/display/components/profiles-group/profiles-group.css"),
    include_str!("../features/display/components/profile-detail/profile-detail.css"),
    include_str!("../features/display/components/trial-pill/trial-pill.css"),
];

/// Distro palette, when Home Manager wrote one. `@define-color` only applies inside
/// this provider, so the file has to be the first sheet rather than a second provider.
fn distro_tokens() -> Option<String> {
    let mut path = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .filter(|dir| !dir.as_os_str().is_empty())
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
        })?;
    path.push("hypr-settings");
    path.push("tokens.css");
    let text = std::fs::read_to_string(&path).ok()?;
    if text.trim().is_empty() {
        return None;
    }
    Some(text)
}

pub fn install() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let tokens = distro_tokens().unwrap_or_else(|| FALLBACK_TOKENS.to_string());
    let mut sheets = vec![tokens.as_str()];
    sheets.extend(SHEETS);
    let provider = gtk::CssProvider::new();
    provider.connect_parsing_error(|_, section, error| {
        tracing::warn!("stylesheet {section}: {error}");
    });
    provider.load_from_string(&sheets.join("\n"));
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        // The HM-generated ~/.config/gtk-4.0/gtk.css loads at USER priority and restyles
        // buttons globally; component styles must beat it to keep chips and banners visible.
        gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
    );
}
