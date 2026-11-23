//! HDR, 10-bit colour and Adaptive Sync: applied through the compositor, verified, kept per
//! profile and put back whenever the profile changes.

use std::time::Duration;

use settings_domain::controls::{
    FeatureCaps, FeatureProbe, Flag, OutputFlags, ProfileFlags, edid_supports_10bit,
    edid_supports_hdr, feature_caps,
};
use settings_domain::{ConnectorName, Output, ProfileName, Topology};

use super::apply_layout::{active_profile, config_for, poll_topology};
use crate::dto::ApplyOutcome;
use crate::error::AppError;
use crate::ports::Ports;

/// Kanshi needs a moment to lay the outputs out before flags can be put back on them.
pub(crate) const SETTLE: Duration = Duration::from_millis(350);
/// Where flags live while no kanshi profile matches.
const FALLBACK_PROFILE: &str = "home";

pub(crate) fn caps_of(ports: &Ports, output: &Output) -> FeatureCaps {
    let drm = ports.probe.drm(&output.connector);
    let edid = drm.edid.as_deref().unwrap_or_default();
    feature_caps(FeatureProbe {
        vrr_capable: drm.vrr_capable,
        max_bpc: drm.max_bpc,
        edid_hdr: edid_supports_hdr(edid),
        edid_10bit: edid_supports_10bit(edid),
        live_hdr: output.hdr,
        live_ten_bit: output.ten_bit,
        live_vrr: output.vrr,
    })
}

/// The profile flags are stored under: the one kanshi selected, or `home`.
pub(crate) fn profile_for_flags(ports: &Ports, topology: &Topology) -> Result<String, AppError> {
    let profiles = ports.profiles.load()?;
    Ok(active_profile(&profiles, topology)
        .map_or_else(|| FALLBACK_PROFILE.to_owned(), |name| name.to_string()))
}

fn apply_flags(ports: &Ports, output: &Output, flags: OutputFlags) -> Result<(), AppError> {
    let mut config = config_for(output, None);
    config.flags = flags;
    ports.live.apply(&config)?;
    Ok(())
}

fn flag_holds(topology: &Topology, connector: &ConnectorName, flag: Flag, value: bool) -> bool {
    topology
        .find(connector)
        .is_some_and(|live| flag.live(live) == value)
}

pub(crate) fn set_flag(
    ports: &Ports,
    connector: &ConnectorName,
    flag: Flag,
    enable: bool,
) -> Result<ApplyOutcome, AppError> {
    let topology = ports.topology.read()?;
    let output = topology
        .find(connector)
        .filter(|o| o.enabled)
        .ok_or_else(|| AppError::Rejected(format!("{connector} is off")))?;

    let caps = caps_of(ports, output);
    let capability = caps.of(flag);
    if enable && !capability.supported {
        return Err(AppError::Rejected(format!(
            "{} is not available on {connector}: {}",
            flag.label(),
            capability.reason
        )));
    }

    let previous = flag.live(output);
    if previous != enable {
        apply_flags(ports, output, OutputFlags::only(flag, enable))?;
        if !poll_topology(ports, |t| flag_holds(t, connector, flag, enable)) {
            if let Err(err) = apply_flags(ports, output, OutputFlags::only(flag, previous)) {
                tracing::warn!(%err, "could not put {} back", flag.label());
            }
            return Err(AppError::Rejected(format!(
                "{connector} did not accept {}; it was put back",
                flag.label()
            )));
        }
    }

    let profile = profile_for_flags(ports, &topology)?;
    let mut stored = ports.flags.load().unwrap_or_default();
    stored
        .entry(profile)
        .or_default()
        .entry(connector.to_string())
        .or_default()
        .set(flag, Some(enable));
    ports.flags.save(&stored)?;
    Ok(ApplyOutcome::Verified)
}

/// Remembers what every connected output is doing now under `profile`.
pub(crate) fn snapshot_flags(
    ports: &Ports,
    profile: &str,
    topology: &Topology,
) -> Result<(), AppError> {
    let mut stored: ProfileFlags = ports.flags.load().unwrap_or_default();
    stored.insert(
        profile.to_owned(),
        topology
            .outputs()
            .iter()
            .map(|o| (o.connector.to_string(), OutputFlags::live(o)))
            .collect(),
    );
    ports.flags.save(&stored)?;
    Ok(())
}

pub(crate) fn forget_profile_flags(ports: &Ports, profile: &ProfileName) {
    let Ok(mut stored) = ports.flags.load() else {
        return;
    };
    if stored.remove(profile.as_str()).is_some()
        && let Err(err) = ports.flags.save(&stored)
    {
        tracing::warn!(%err, "could not drop the flags of {profile}");
    }
}

/// Puts the stored flags of `profile` back on the connected outputs. Enabling something an
/// output cannot do is skipped. Returns how many outputs were changed.
pub(crate) fn restore_flags_for(
    ports: &Ports,
    profile: &str,
    settle: bool,
) -> Result<usize, AppError> {
    let stored = ports.flags.load()?;
    let Some(by_connector) = stored.get(profile).filter(|m| !m.is_empty()) else {
        return Ok(0);
    };
    if settle {
        ports.clock.sleep(SETTLE);
    }
    let topology = ports.topology.read()?;
    let mut changed = 0;
    for output in topology.outputs().iter().filter(|o| o.enabled) {
        let Some(wanted) = by_connector.get(output.connector.as_str()) else {
            continue;
        };
        let caps = caps_of(ports, output);
        let mut patch = OutputFlags::default();
        for flag in Flag::ALL {
            let Some(value) = wanted.get(flag) else {
                continue;
            };
            if flag.live(output) == value {
                continue;
            }
            if value && !caps.of(flag).supported {
                tracing::warn!(
                    "{} is not available on {}; skipped",
                    flag.label(),
                    output.connector
                );
                continue;
            }
            patch.set(flag, Some(value));
        }
        if patch.is_empty() {
            continue;
        }
        match apply_flags(ports, output, patch) {
            Ok(()) => changed += 1,
            Err(err) => tracing::warn!(%err, "restoring flags on {} failed", output.connector),
        }
    }
    Ok(changed)
}

pub(crate) fn restore_flags(ports: &Ports) -> Result<usize, AppError> {
    let topology = ports.topology.read()?;
    let profile = profile_for_flags(ports, &topology)?;
    restore_flags_for(ports, &profile, true)
}
