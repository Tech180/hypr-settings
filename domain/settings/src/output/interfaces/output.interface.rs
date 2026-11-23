use crate::modes::AdvertisedModes;
use crate::value::{ConnectorName, Mode, Position, Scale, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayKind {
    Laptop,
    Monitor,
    Tv,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub connector: ConnectorName,
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub kind: DisplayKind,
    /// `None` while the output is disabled and the compositor reports no geometry.
    pub mode: Option<Mode>,
    pub position: Position,
    pub scale: Scale,
    pub transform: Transform,
    pub enabled: bool,
    pub focused: bool,
    pub modes: AdvertisedModes,
    pub vrr: bool,
    pub hdr: bool,
    pub ten_bit: bool,
}

/// What a kanshi profile can recognise a connected display by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputIdentity {
    pub connector: ConnectorName,
    pub make: String,
    pub model: String,
    pub serial: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Topology(pub(crate) Vec<Output>);
