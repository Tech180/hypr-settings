use crate::controls::OutputFlags;
use crate::value::{ConnectorName, Mode, Position, Scale, Transform};

/// A direct compositor configuration for one output.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputConfig {
    pub connector: ConnectorName,
    pub mode: Option<Mode>,
    pub position: Position,
    pub scale: Scale,
    pub transform: Transform,
    pub enabled: bool,
    /// HDR, 10-bit and Adaptive Sync; `None` fields are left as the compositor has them.
    pub flags: OutputFlags,
}
