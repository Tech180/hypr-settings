use thiserror::Error;

use crate::value::{ConnectorName, Scale};

#[derive(Debug, Clone, PartialEq, Error)]
pub enum LayoutError {
    #[error("unknown output {0}")]
    UnknownOutput(ConnectorName),
    #[error("{request} is not advertised by {connector}")]
    UnsupportedMode {
        connector: ConnectorName,
        request: String,
    },
    #[error("{0} has no current mode to derive the request from")]
    NoCurrentMode(ConnectorName),
    #[error("cannot disable {0}: it is the last enabled display")]
    LastDisplay(ConnectorName),
    #[error("{0} hosts the bar: make another display the main one first")]
    BarHost(ConnectorName),
    #[error("{first} and {second} would overlap")]
    Overlap {
        first: ConnectorName,
        second: ConnectorName,
    },
    #[error(
        "scale {requested} does not divide {connector}'s resolution into whole pixels (try {nearest})"
    )]
    InvalidScale {
        connector: ConnectorName,
        requested: Scale,
        nearest: Scale,
    },
    #[error("{0} has no usable mode to size its place in the layout")]
    NoLayoutSize(ConnectorName),
}
