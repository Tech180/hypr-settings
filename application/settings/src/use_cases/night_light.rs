use settings_domain::Kelvin;

use crate::dto::NightLightView;
use crate::error::AppError;
use crate::ports::Ports;

pub(crate) fn view(ports: &Ports) -> NightLightView {
    let prefs = ports.prefs.load().unwrap_or_default();
    NightLightView {
        on: prefs.night_override,
        kelvin: Kelvin::clamped(prefs.night_kelvin).get(),
    }
}

pub(crate) fn set_on(ports: &Ports, on: bool) -> Result<(), AppError> {
    let mut prefs = ports.prefs.load().unwrap_or_default();
    let kelvin = Kelvin::clamped(prefs.night_kelvin);
    if on {
        ports.night.enable(kelvin)?;
    } else {
        ports.night.disable()?;
    }
    prefs.night_override = on;
    prefs.night_kelvin = kelvin.get();
    ports.prefs.save(&prefs)?;
    Ok(())
}

pub(crate) fn set_kelvin(ports: &Ports, kelvin: u32) -> Result<(), AppError> {
    let mut prefs = ports.prefs.load().unwrap_or_default();
    let kelvin = Kelvin::clamped(kelvin);
    if prefs.night_override {
        ports.night.enable(kelvin)?;
    }
    prefs.night_kelvin = kelvin.get();
    ports.prefs.save(&prefs)?;
    Ok(())
}

pub(crate) fn restore(ports: &Ports) -> Result<(), AppError> {
    let prefs = ports.prefs.load().unwrap_or_default();
    if prefs.night_override {
        ports.night.enable(Kelvin::clamped(prefs.night_kelvin))?;
    }
    Ok(())
}
