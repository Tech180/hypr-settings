use std::rc::Rc;

use gtk4 as gtk;

/// One display, in layout space (logical pixels).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileProps {
    pub connector: String,
    pub size_label: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub enabled: bool,
    pub focused: bool,
    pub selected: bool,
}

/// The layout after a drop: `(connector, wanted position, snap threshold)` in, every
/// enabled display's `(connector, x, y)` out, with the dragged one at its legal spot.
/// `None` when the tile cannot be placed.
pub type PreviewDrop = Rc<dyn Fn(&str, (i32, i32), i32) -> Option<Vec<(String, i32, i32)>>>;

#[derive(Clone)]
pub struct ArrangementCanvasProps {
    pub tiles: Vec<TileProps>,
    pub on_select: Rc<dyn Fn(String)>,
    /// A tile was dropped somewhere new: connector and its new layout position.
    pub on_drop: Rc<dyn Fn(String, i32, i32)>,
    pub preview: PreviewDrop,
}

pub struct TileWidgets {
    pub connector: String,
    pub root: gtk::Box,
    pub badge: gtk::Label,
    pub label: gtk::Label,
    pub resolution: gtk::Label,
}

pub struct CanvasWidgets {
    pub root: gtk::Overlay,
    pub backdrop: gtk::DrawingArea,
    pub fixed: gtk::Fixed,
    pub ghost: gtk::Box,
    pub hint: gtk::Label,
}
