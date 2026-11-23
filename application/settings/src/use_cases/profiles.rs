use settings_domain::{ConnectorName, HOME_PROFILE, LayoutPatch, LayoutPolicy, ProfileName};

use super::apply_layout::active_profile;
use super::flags::{forget_profile_flags, restore_flags_for, snapshot_flags};

use super::apply_layout::apply_layout;
use crate::dto::ApplyOutcome;
use crate::error::AppError;
use crate::ports::Ports;

/// Immediate (trial-free) enable or disable of one output.
pub(crate) fn set_enabled(
    ports: &Ports,
    connector: ConnectorName,
    enable: bool,
) -> Result<ApplyOutcome, AppError> {
    let topology = ports.topology.read()?;
    let prefs = ports.prefs.load().unwrap_or_default();
    let patch = LayoutPatch::new(connector).enable(enable);
    let bar_host = prefs.bar_host_connector.clone().or_else(|| {
        topology
            .bar_host(prefs.bar_host_connector.as_deref())
            .map(|o| o.connector.to_string())
    });
    let last_display = patch.enable == Some(false) && topology.enabled_count() <= 1;
    if !last_display {
        LayoutPolicy::guard_bar_host(&topology, &patch, bar_host.as_deref())?;
    }
    let resolved = LayoutPolicy::validate(&topology, &patch)?;
    apply_layout(ports, &topology, &resolved)
}

pub(crate) fn switch_profile(ports: &Ports, name: &ProfileName) -> Result<(), AppError> {
    ports.controller.switch(name)?;
    ports.controller.reload()?;
    let _ = restore_flags_for(ports, name.as_str(), true)?;
    Ok(())
}

pub(crate) fn save_snapshot_as(ports: &Ports, name: &ProfileName) -> Result<(), AppError> {
    let topology = ports.topology.read()?;
    let mut profiles = ports.profiles.load()?;
    if !profiles.snapshot_to_profile(name, &topology) {
        return Err(AppError::Rejected(format!(
            "nothing to store in profile {name}"
        )));
    }
    ports.profiles.save(&profiles)?;
    snapshot_flags(ports, name.as_str(), &topology)?;
    ports.controller.reload()?;
    Ok(())
}

pub(crate) fn save_to_active_profile(ports: &Ports) -> Result<String, AppError> {
    let topology = ports.topology.read()?;
    let mut profiles = ports.profiles.load()?;
    let name = active_profile(&profiles, &topology).unwrap_or_else(|| {
        ProfileName::from_stored(HOME_PROFILE).expect("static profile name is valid")
    });
    if !profiles.snapshot_to_profile(&name, &topology) {
        return Err(AppError::Rejected(
            "nothing to store in the active profile".into(),
        ));
    }
    ports.profiles.save(&profiles)?;
    Ok(name.to_string())
}

pub(crate) fn delete_profile(ports: &Ports, name: &ProfileName) -> Result<(), AppError> {
    if name.is_default() {
        return Err(AppError::Rejected(
            "the default profile cannot be removed".into(),
        ));
    }
    let mut profiles = ports.profiles.load()?;
    let before = profiles.profiles.len();
    profiles.profiles.retain(|p| p.name != *name);
    if profiles.profiles.len() == before {
        return Err(AppError::Rejected(format!("unknown profile {name}")));
    }
    ports.profiles.save(&profiles)?;
    forget_profile_flags(ports, name);
    Ok(())
}
