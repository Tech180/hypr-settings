use settings_domain::ConnectorName;
use settings_domain::DisplayPreferences;

use crate::error::AppError;
use crate::ports::Ports;

pub(crate) fn set_bar_host(ports: &Ports, connector: ConnectorName) -> Result<(), AppError> {
    let topology = ports.topology.read()?;
    if !topology.find(&connector).is_some_and(|o| o.enabled) {
        return Err(AppError::Rejected(format!(
            "{connector} is off, so it cannot host the bar"
        )));
    }
    let mut prefs = ports.prefs.load().unwrap_or_default();
    if prefs.bar_host_connector.as_deref() == Some(connector.as_str()) {
        return Ok(());
    }
    prefs.bar_host_connector = Some(connector.to_string());
    ports.prefs.save(&prefs)?;
    super::controls::refresh_host(ports);
    Ok(())
}

pub(crate) fn add_custom_hz(
    ports: &Ports,
    connector: &ConnectorName,
    hz: u32,
) -> Result<(), AppError> {
    if !DisplayPreferences::is_valid_custom_hz(hz) {
        return Err(AppError::Rejected(format!(
            "{hz} Hz is not a usable refresh rate (1 to {})",
            settings_domain::controls::MAX_CUSTOM_HZ
        )));
    }
    let mut prefs = ports.prefs.load().unwrap_or_default();
    if prefs.add_custom_hz(connector.as_str(), hz) {
        ports.prefs.save(&prefs)?;
    }
    Ok(())
}

pub(crate) fn remove_custom_hz(
    ports: &Ports,
    connector: &ConnectorName,
    hz: u32,
) -> Result<(), AppError> {
    let mut prefs = ports.prefs.load().unwrap_or_default();
    if prefs.remove_custom_hz(connector.as_str(), hz) {
        ports.prefs.save(&prefs)?;
    }
    Ok(())
}
