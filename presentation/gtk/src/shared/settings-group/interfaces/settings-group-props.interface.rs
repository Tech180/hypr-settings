use std::rc::Rc;

use crate::shared::badge::BadgeProps;
use crate::shared::icon_tile::IconTileProps;
use crate::shared::segmented::SegmentedProps;
pub use crate::shared::slider::SliderProps;

/// Rates the user added, each removable, and a field to add another.
#[derive(Clone)]
pub struct RateChipsProps {
    pub rates: Vec<u32>,
    pub on_add: Rc<dyn Fn(u32)>,
    pub on_remove: Rc<dyn Fn(u32)>,
}

#[derive(Clone)]
pub enum RowControl {
    Switch {
        active: bool,
        on_toggle: Rc<dyn Fn(bool)>,
    },
    Select {
        options: Vec<String>,
        selected: usize,
        on_select: Rc<dyn Fn(usize)>,
    },
    Segmented(SegmentedProps),
    Slider(SliderProps),
    Button {
        label: String,
        on_click: Rc<dyn Fn()>,
    },
    #[allow(dead_code)]
    RateChips(RateChipsProps),
    /// A read-only fact shown as a pill.
    Pill(BadgeProps),
    /// The whole row opens something; a chevron says so.
    Chevron(Rc<dyn Fn()>),
    /// Accent section label row with no trailing control.
    Section,
}

#[derive(Clone)]
pub struct SettingRowProps {
    pub title: String,
    pub subtitle: Option<String>,
    /// A row that cannot be used right now is dimmed; the subtitle says why.
    pub sensitive: bool,
    pub control: RowControl,
    /// A glyph tile before the title.
    pub leading: Option<IconTileProps>,
    /// A pill next to the title, such as "Experimental".
    pub badge: Option<BadgeProps>,
}

#[derive(Clone)]
pub struct SettingsGroupProps {
    pub rows: Vec<SettingRowProps>,
}
