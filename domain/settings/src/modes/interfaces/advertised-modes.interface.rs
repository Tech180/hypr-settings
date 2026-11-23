use crate::value::Mode;

/// Modes an output advertises, in compositor order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdvertisedModes(pub(crate) Vec<Mode>);
