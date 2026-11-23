use std::time::{Duration, Instant};

use super::output_config::OutputConfig;
use super::subscription::Subscription;
use super::topology_event::TopologyEvent;
use crate::error::PortError;
use crate::output::Topology;
use crate::preferences::{DisplayPreferences, TrialRecord};
use crate::profile::{ProfileName, ProfileSet};

pub trait TopologyReader: Send + Sync {
    fn read(&self) -> Result<Topology, PortError>;
}

pub trait TopologyEvents: Send + Sync {
    /// Calls `sink` (debounced by the adapter) when outputs may have changed.
    fn subscribe(&self, sink: Box<dyn Fn(TopologyEvent) + Send>)
    -> Result<Subscription, PortError>;
}

pub trait ProfileStore: Send + Sync {
    /// Bootstraps from a seed when nothing is stored yet.
    fn load(&self) -> Result<ProfileSet, PortError>;
    /// Atomic: readers never observe a partial file.
    fn save(&self, set: &ProfileSet) -> Result<(), PortError>;
}

pub trait ProfileController: Send + Sync {
    fn reload(&self) -> Result<(), PortError>;
    fn switch(&self, name: &ProfileName) -> Result<(), PortError>;
}

pub trait LiveOutputs: Send + Sync {
    /// Direct compositor apply (fallback when kanshi is down).
    fn apply(&self, config: &OutputConfig) -> Result<(), PortError>;
}

pub trait PreferencesStore: Send + Sync {
    fn load(&self) -> Result<DisplayPreferences, PortError>;
    fn save(&self, prefs: &DisplayPreferences) -> Result<(), PortError>;
}

pub trait TrialJournal: Send + Sync {
    fn write(&self, record: &TrialRecord) -> Result<(), PortError>;
    fn read(&self) -> Result<Option<TrialRecord>, PortError>;
    fn clear(&self) -> Result<(), PortError>;
}

pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    fn unix_secs(&self) -> u64;
    fn sleep(&self, duration: Duration);
}
