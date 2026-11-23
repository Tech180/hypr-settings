use crate::value::ConnectorName;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    pub connector: ConnectorName,
    pub rect: Rect,
}

/// The enabled outputs of a topology as rectangles.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layout {
    pub(crate) tiles: Vec<Tile>,
}
