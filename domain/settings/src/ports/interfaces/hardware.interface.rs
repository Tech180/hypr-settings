use crate::ConnectorName;
use crate::controls::interfaces::DrmProbe;
use crate::controls::{BrightnessSnapshot, DdcDisplay, InputSources, ProfileFlags};
use crate::error::PortError;
use crate::night_light::Kelvin;

pub trait Backlight: Send + Sync {
    fn read(&self) -> Result<Option<u8>, PortError>;
    fn set(&self, percent: u8) -> Result<(), PortError>;
}

pub trait Ddc: Send + Sync {
    fn detect(&self, force: bool) -> Vec<DdcDisplay>;
    fn read_percent(&self, bus: u32, code: u8) -> Option<u8>;
    fn write_percent(&self, bus: u32, code: u8, percent: u8) -> Result<(), PortError>;
    fn overdrive_code(&self, bus: u32) -> Option<u8>;
    fn input_sources(&self, bus: u32) -> InputSources;
    fn set_input_source(&self, bus: u32, code: u8) -> Result<(), PortError>;
}

pub trait NightLight: Send + Sync {
    fn enable(&self, kelvin: Kelvin) -> Result<(), PortError>;
    fn disable(&self) -> Result<(), PortError>;
}

pub trait HardwareProbe: Send + Sync {
    fn drm(&self, connector: &ConnectorName) -> DrmProbe;
}

pub trait FeatureFlagStore: Send + Sync {
    fn load(&self) -> Result<ProfileFlags, PortError>;
    fn save(&self, flags: &ProfileFlags) -> Result<(), PortError>;
}

pub trait BrightnessState: Send + Sync {
    fn write(&self, snapshot: &BrightnessSnapshot) -> Result<(), PortError>;
    fn read(&self) -> Result<Option<BrightnessSnapshot>, PortError>;
}
