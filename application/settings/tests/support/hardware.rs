use std::sync::Arc;

use settings_domain::controls::{
    BrightnessSnapshot, DdcDisplay, DrmProbe, InputSource, InputSources, ProfileFlags,
    percent_from_raw, raw_from_percent,
};
use settings_domain::ports::{
    Backlight, BrightnessState, Ddc, FeatureFlagStore, HardwareProbe, NightLight,
};
use settings_domain::{ConnectorName, Kelvin, PortError};

use super::Fake;

pub fn ddc_display(bus: u32, connector: &str) -> DdcDisplay {
    DdcDisplay {
        bus,
        connector: Some(connector.into()),
        model: String::new(),
        serial: String::new(),
    }
}

pub fn input(label: &str, code: u8) -> InputSource {
    InputSource {
        label: label.into(),
        code,
    }
}

impl Fake {
    pub fn hardware_ports(self: &Arc<Self>) -> settings_application::Ports {
        settings_application::Ports {
            topology: self.clone(),
            events: self.clone(),
            profiles: self.clone(),
            controller: self.clone(),
            live: self.clone(),
            prefs: self.clone(),
            journal: self.clone(),
            clock: self.clone(),
            flags: self.clone(),
            probe: self.clone(),
            backlight: self.clone(),
            ddc: self.clone(),
            night: self.clone(),
            brightness: self.clone(),
        }
    }
}

impl Backlight for Fake {
    fn read(&self) -> Result<Option<u8>, PortError> {
        Ok(self.with(|w| w.backlight))
    }

    fn set(&self, percent: u8) -> Result<(), PortError> {
        self.with(|w| w.backlight = Some(percent));
        Ok(())
    }
}

impl Ddc for Fake {
    fn detect(&self, _: bool) -> Vec<DdcDisplay> {
        self.with(|w| w.ddc_displays.clone())
    }

    fn read_percent(&self, bus: u32, code: u8) -> Option<u8> {
        self.with(|w| {
            let raw = *w.ddc_values.get(&(bus, code))?;
            let max = w.ddc_max.get(&(bus, code)).copied().unwrap_or(100);
            percent_from_raw(raw, max)
        })
    }

    fn write_percent(&self, bus: u32, code: u8, percent: u8) -> Result<(), PortError> {
        self.with(|w| {
            let max = w.ddc_max.get(&(bus, code)).copied().unwrap_or(100);
            w.ddc_values
                .insert((bus, code), raw_from_percent(percent, max));
        });
        Ok(())
    }

    fn overdrive_code(&self, bus: u32) -> Option<u8> {
        let _ = bus;
        self.with(|w| w.overdrive_code)
    }

    fn input_sources(&self, _: u32) -> InputSources {
        self.with(|w| w.inputs.clone())
    }

    fn set_input_source(&self, bus: u32, code: u8) -> Result<(), PortError> {
        self.with(|w| {
            w.log.push(format!("ddc.input {bus} {code:#x}"));
            w.ddc_values.insert((bus, 0x60), u32::from(code));
        });
        Ok(())
    }
}

impl HardwareProbe for Fake {
    fn drm(&self, connector: &ConnectorName) -> DrmProbe {
        self.with(|w| w.drm.get(connector.as_str()).cloned().unwrap_or_default())
    }
}

impl FeatureFlagStore for Fake {
    fn load(&self) -> Result<ProfileFlags, PortError> {
        Ok(self.with(|w| w.flags.clone()))
    }

    fn save(&self, flags: &ProfileFlags) -> Result<(), PortError> {
        self.with(|w| w.flags = flags.clone());
        Ok(())
    }
}

impl BrightnessState for Fake {
    fn write(&self, snapshot: &BrightnessSnapshot) -> Result<(), PortError> {
        self.with(|w| w.brightness_state = Some(snapshot.clone()));
        Ok(())
    }

    fn read(&self) -> Result<Option<BrightnessSnapshot>, PortError> {
        Ok(self.with(|w| w.brightness_state.clone()))
    }
}

impl NightLight for Fake {
    fn enable(&self, kelvin: Kelvin) -> Result<(), PortError> {
        self.with(|w| w.night = Some(kelvin));
        Ok(())
    }

    fn disable(&self) -> Result<(), PortError> {
        self.with(|w| w.night = None);
        Ok(())
    }
}
