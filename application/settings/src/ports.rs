use std::sync::Arc;

use settings_domain::ports::{
    Backlight, BrightnessState, Clock, Ddc, FeatureFlagStore, HardwareProbe, LiveOutputs,
    NightLight, PreferencesStore, ProfileController, ProfileStore, TopologyEvents, TopologyReader,
    TrialJournal,
};

/// Every driven port the use cases need, shared behind `Arc<dyn _>` so adapters and
/// test fakes plug in the same way.
#[derive(Clone)]
pub struct Ports {
    pub topology: Arc<dyn TopologyReader>,
    pub events: Arc<dyn TopologyEvents>,
    pub profiles: Arc<dyn ProfileStore>,
    pub controller: Arc<dyn ProfileController>,
    pub live: Arc<dyn LiveOutputs>,
    pub prefs: Arc<dyn PreferencesStore>,
    pub journal: Arc<dyn TrialJournal>,
    pub clock: Arc<dyn Clock>,
    pub flags: Arc<dyn FeatureFlagStore>,
    pub probe: Arc<dyn HardwareProbe>,
    pub backlight: Arc<dyn Backlight>,
    pub ddc: Arc<dyn Ddc>,
    pub night: Arc<dyn NightLight>,
    pub brightness: Arc<dyn BrightnessState>,
}
