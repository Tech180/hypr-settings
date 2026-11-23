use crate::value::{ConnectorName, Mode, Position, Resolution, Scale, Transform};

/// How a caller names the mode it wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeRequest {
    Exact(Mode),
    /// Change resolution, keeping the current rate when the new resolution supports it.
    Resolution(Resolution),
    /// Change the rounded refresh rate at the current resolution.
    Refresh(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutPatch {
    pub connector: ConnectorName,
    pub mode: Option<ModeRequest>,
    pub scale: Option<Scale>,
    pub transform: Option<Transform>,
    pub enable: Option<bool>,
    /// Where the output's top-left corner should go, in logical pixels.
    pub position: Option<Position>,
}

/// Something worth telling the user about a layout that is still legal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutWarning {
    /// Some monitors share no edge with the rest, so the pointer cannot reach them.
    Disconnected,
}

/// A validated patch: any mode request is already a concrete advertised `Mode`.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPatch {
    pub connector: ConnectorName,
    pub mode: Option<Mode>,
    pub scale: Option<Scale>,
    pub transform: Option<Transform>,
    pub enable: Option<bool>,
    /// Final position of the patched output, when it ends up enabled.
    pub position: Option<Position>,
    /// Final position of every enabled output once the patch and its reflow are
    /// applied. Persisting all of them keeps the stored layout consistent.
    pub positions: Vec<(ConnectorName, Position)>,
    pub warnings: Vec<LayoutWarning>,
}

pub struct LayoutPolicy;
