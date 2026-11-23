use gtk4 as gtk;
use libadwaita as adw;

use crate::features::display::components::arrangement_canvas::ArrangementCanvasProps;
use crate::features::display::components::display_dashboard::DisplayDashboardProps;
use crate::features::display::components::profiles_group::ProfilesGroupProps;
use crate::shared::section_header::SectionHeaderProps;

#[derive(Clone)]
pub struct DisplayPageProps {
    pub arrangement: ArrangementCanvasProps,
    /// A layout problem worth showing, such as a display the pointer cannot reach.
    pub notice: Option<String>,
    pub output_header: SectionHeaderProps,
    pub dashboard: DisplayDashboardProps,
    pub profiles: ProfilesGroupProps,
}

/// The stable skeleton of the page: a stack of the main view and the profile sub page, with
/// the trial pill floating above. The arrangement section is built once so the canvas can
/// animate between layouts; only the sections below it (`rest`) and the sub page are
/// rebuilt on every refresh.
pub struct PageFrame {
    pub root: gtk::Overlay,
    pub stack: gtk::Stack,
    pub scroller: gtk::ScrolledWindow,
    pub body: adw::Bin,
    /// The main column, shown in `body` whenever there are displays.
    pub content: gtk::Widget,
    pub notice: gtk::Label,
    pub rest: adw::Bin,
    pub detail: adw::Bin,
}
