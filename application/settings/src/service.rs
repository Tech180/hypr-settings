use std::sync::Mutex;

use settings_domain::controls::Flag;
use settings_domain::ports::{Subscription, TopologyEvent};
use settings_domain::{ConnectorName, LayoutPatch, ProfileName, TrialPolicy};

use crate::api::DisplayApi;
use crate::dto::{
    ApplyOutcome, BrightnessView, DisplayView, NightLightView, OutputControls, RecoverOutcome,
    TickOutcome, TrialView,
};
use crate::error::AppError;
use crate::ports::Ports;
use crate::use_cases::trial::{self, Applying, TrialState, current_view, ensure_idle, lock};
use crate::use_cases::{
    controls, flags, night_light, preferences, profiles, refresh_view::refresh_view,
};

pub struct DisplayService {
    ports: Ports,
    policy: TrialPolicy,
    state: Mutex<TrialState>,
    /// Serialises writes so a tick, a click and a profile switch never interleave.
    writes: Mutex<()>,
}

impl DisplayService {
    pub fn new(ports: Ports, policy: TrialPolicy) -> Self {
        Self {
            ports,
            policy,
            state: Mutex::default(),
            writes: Mutex::new(()),
        }
    }
}

impl DisplayApi for DisplayService {
    fn view(&self) -> Result<DisplayView, AppError> {
        refresh_view(&self.ports, current_view(&self.ports, &self.state))
    }

    fn begin_trial(&self, patch: LayoutPatch) -> Result<TrialView, AppError> {
        let _applying = Applying::start(&self.state)?;
        let _write = lock(&self.writes);
        trial::begin(&self.ports, self.policy, &self.state, patch)
    }

    fn keep_trial(&self) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        trial::keep(&self.ports, &self.state)
    }

    fn revert_trial(&self) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        trial::revert(&self.ports, &self.state)
    }

    fn tick(&self) -> Result<TickOutcome, AppError> {
        let _write = lock(&self.writes);
        trial::tick(&self.ports, &self.state)
    }

    fn set_enabled(
        &self,
        connector: ConnectorName,
        enable: bool,
    ) -> Result<ApplyOutcome, AppError> {
        let _applying = Applying::start(&self.state)?;
        let _write = lock(&self.writes);
        profiles::set_enabled(&self.ports, connector, enable)
    }

    fn switch_profile(&self, name: ProfileName) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        ensure_idle(&self.state)?;
        profiles::switch_profile(&self.ports, &name)
    }

    fn save_snapshot_as(&self, name: ProfileName) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        ensure_idle(&self.state)?;
        profiles::save_snapshot_as(&self.ports, &name)
    }

    fn save_to_active_profile(&self) -> Result<String, AppError> {
        let _write = lock(&self.writes);
        ensure_idle(&self.state)?;
        profiles::save_to_active_profile(&self.ports)
    }

    fn set_bar_host(&self, connector: ConnectorName) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        preferences::set_bar_host(&self.ports, connector)
    }

    fn add_custom_hz(&self, connector: ConnectorName, hz: u32) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        preferences::add_custom_hz(&self.ports, &connector, hz)
    }

    fn remove_custom_hz(&self, connector: ConnectorName, hz: u32) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        preferences::remove_custom_hz(&self.ports, &connector, hz)
    }

    fn delete_profile(&self, name: ProfileName) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        ensure_idle(&self.state)?;
        profiles::delete_profile(&self.ports, &name)
    }

    fn recover_pending_trial(&self) -> Result<RecoverOutcome, AppError> {
        let _write = lock(&self.writes);
        trial::recover(&self.ports, &self.state)
    }

    fn subscribe(&self, sink: Box<dyn Fn(TopologyEvent) + Send>) -> Result<Subscription, AppError> {
        Ok(self.ports.events.subscribe(sink)?)
    }
}

impl crate::api::ControlsApi for DisplayService {
    fn output_controls(&self, connector: ConnectorName) -> Result<OutputControls, AppError> {
        let _write = lock(&self.writes);
        controls::output_controls(&self.ports, &connector)
    }

    fn brightness(&self) -> Result<BrightnessView, AppError> {
        let _write = lock(&self.writes);
        controls::brightness(&self.ports)
    }

    fn refresh_brightness(&self) -> Result<BrightnessView, AppError> {
        let _write = lock(&self.writes);
        controls::refresh_brightness(&self.ports)
    }

    fn set_brightness(&self, percent: u8) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        controls::set_brightness(&self.ports, percent)
    }

    fn adjust_brightness(&self, delta: i32) -> Result<u8, AppError> {
        let _write = lock(&self.writes);
        controls::adjust_brightness(&self.ports, delta)
    }

    fn set_sync_brightness(&self, sync: bool) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        controls::set_sync(&self.ports, sync)
    }

    fn set_contrast(&self, connector: ConnectorName, percent: u8) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        controls::set_contrast(&self.ports, &connector, percent)
    }

    fn set_overdrive(&self, connector: ConnectorName, percent: u8) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        controls::set_overdrive(&self.ports, &connector, percent)
    }

    fn set_input_source(&self, connector: ConnectorName, code: u8) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        controls::set_input_source(&self.ports, &connector, code)
    }

    fn set_flag(
        &self,
        connector: ConnectorName,
        flag: Flag,
        enable: bool,
    ) -> Result<ApplyOutcome, AppError> {
        let _write = lock(&self.writes);
        flags::set_flag(&self.ports, &connector, flag, enable)
    }

    fn restore_flags(&self) -> Result<usize, AppError> {
        let _write = lock(&self.writes);
        flags::restore_flags(&self.ports)
    }

    fn night_light(&self) -> Result<NightLightView, AppError> {
        Ok(night_light::view(&self.ports))
    }

    fn set_night_light(&self, on: bool) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        night_light::set_on(&self.ports, on)
    }

    fn set_night_kelvin(&self, kelvin: u32) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        night_light::set_kelvin(&self.ports, kelvin)
    }

    fn restore_night_light(&self) -> Result<(), AppError> {
        let _write = lock(&self.writes);
        night_light::restore(&self.ports)
    }
}
