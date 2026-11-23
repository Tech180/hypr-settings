use gtk4 as gtk;
use libadwaita as adw;

pub struct ShellProps {
    pub app: adw::Application,
    pub toasts: adw::ToastOverlay,
    pub sidebar: adw::NavigationPage,
    /// `(section id, content)` pairs stacked in the content area.
    pub pages: Vec<(String, gtk::Widget)>,
}

pub struct ShellWidgets {
    pub window: adw::ApplicationWindow,
    pub stack: gtk::Stack,
    pub split: adw::NavigationSplitView,
    pub content_page: adw::NavigationPage,
}
