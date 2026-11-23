use std::time::Duration;

use settings_domain::controls::OutputFlags;
use settings_domain::output::logical_size;
use settings_domain::ports::OutputConfig;
use settings_domain::{
    ConnectorName, Mode, Output, Position, ProfileName, ProfileSet, ResolvedPatch, Scale, Topology,
    Transform,
};

use crate::dto::ApplyOutcome;
use crate::error::AppError;
use crate::ports::Ports;

const POLLS: usize = 8;
const FIRST_POLL: Duration = Duration::from_millis(200);
const NEXT_POLL: Duration = Duration::from_millis(250);
/// Hyprland reports layout positions exactly, but allow a pixel like other tools do.
const POSITION_TOLERANCE: i32 = 1;
/// Hyprland reports the scale it really uses, which can differ in the last digits.
const SCALE_TOLERANCE: f64 = 0.02;

/// What the compositor should report for one output once a change has landed.
struct Expectation {
    connector: ConnectorName,
    mode: Option<Mode>,
    scale: Option<Scale>,
    transform: Option<Transform>,
    enabled: Option<bool>,
    position: Option<Position>,
}

impl Expectation {
    /// The patched output must show every requested field; the rest of the layout
    /// is checked through [`Self::positions_of_patch`].
    fn of_patch(patch: &ResolvedPatch) -> Self {
        Self {
            connector: patch.connector.clone(),
            mode: patch.mode,
            scale: patch.scale,
            transform: patch.transform,
            enabled: patch.enable,
            position: patch.position,
        }
    }

    /// Every other enabled output must have been placed where the reflow said.
    fn positions_of_patch(patch: &ResolvedPatch, topology: &Topology) -> Vec<Self> {
        patch
            .positions
            .iter()
            .filter(|(connector, _)| *connector != patch.connector)
            .filter(|(connector, _)| topology.find(connector).is_some_and(|o| o.enabled))
            .map(|(connector, position)| Self {
                connector: connector.clone(),
                mode: None,
                scale: None,
                transform: None,
                enabled: None,
                position: Some(*position),
            })
            .collect()
    }

    fn of_output(output: &Output) -> Self {
        Self {
            connector: output.connector.clone(),
            mode: output.mode,
            scale: Some(output.scale),
            transform: Some(output.transform),
            enabled: Some(output.enabled),
            position: output.enabled.then_some(output.position),
        }
    }

    fn met_by(&self, live: &Output) -> bool {
        let mode_ok = self.mode.is_none_or(|want| {
            live.mode.is_some_and(|got| {
                got.resolution == want.resolution
                    && got.refresh.rounded_hz() == want.refresh.rounded_hz()
            })
        });
        let position_ok = self.position.is_none_or(|want| {
            !live.enabled
                || ((live.position.x - want.x).abs() <= POSITION_TOLERANCE
                    && (live.position.y - want.y).abs() <= POSITION_TOLERANCE)
        });
        mode_ok
            && position_ok
            && self
                .scale
                .is_none_or(|s| (s.as_f64() - live.scale.as_f64()).abs() <= SCALE_TOLERANCE)
            && self.transform.is_none_or(|t| t == live.transform)
            && self.enabled.is_none_or(|e| e == live.enabled)
    }
}

fn verify(ports: &Ports, expectations: &[Expectation]) -> bool {
    for poll in 0..POLLS {
        ports
            .clock
            .sleep(if poll == 0 { FIRST_POLL } else { NEXT_POLL });
        let Ok(topology) = ports.topology.read() else {
            continue;
        };
        if expectations.iter().all(|expect| {
            topology
                .find(&expect.connector)
                .is_some_and(|live| expect.met_by(live))
        }) {
            return true;
        }
    }
    false
}

/// Polls the compositor until `ok` holds for the current topology.
pub(crate) fn poll_topology(ports: &Ports, mut ok: impl FnMut(&Topology) -> bool) -> bool {
    for poll in 0..POLLS {
        ports
            .clock
            .sleep(if poll == 0 { FIRST_POLL } else { NEXT_POLL });
        let Ok(topology) = ports.topology.read() else {
            continue;
        };
        if ok(&topology) {
            return true;
        }
    }
    false
}

pub(crate) fn config_for(output: &Output, patch: Option<&ResolvedPatch>) -> OutputConfig {
    let own = patch.filter(|p| p.connector == output.connector);
    OutputConfig {
        connector: output.connector.clone(),
        mode: own.and_then(|p| p.mode).or(output.mode),
        position: patch
            .and_then(|p| p.position_of(&output.connector))
            .unwrap_or(output.position),
        scale: own.and_then(|p| p.scale).unwrap_or(output.scale),
        transform: own.and_then(|p| p.transform).unwrap_or(output.transform),
        enabled: own.and_then(|p| p.enable).unwrap_or(output.enabled),
        flags: OutputFlags::live(output),
    }
}

/// Disabled outputs have no place in the compositor's layout, so the profile's
/// saved position is the better guess for where they come back. Looks the saved
/// position up through the matching profile and overlays it.
pub(crate) fn with_saved_positions(
    topology: &Topology,
    profiles: &ProfileSet,
    active: Option<&ProfileName>,
) -> Topology {
    let Some(active) = active else {
        return topology.clone();
    };
    let slots = profiles.slots_for(active, topology);
    let outputs = topology
        .outputs()
        .iter()
        .map(|output| {
            let saved = slots
                .iter()
                .find(|(_, live)| live.is_some_and(|l| l.connector == output.connector))
                .and_then(|(slot, _)| slot.position);
            match saved {
                Some(position) if !output.enabled => Output {
                    position,
                    ..output.clone()
                },
                _ => output.clone(),
            }
        })
        .collect();
    Topology::new(outputs)
}

/// Reads the live topology and validates `patch` against it, with disabled outputs
/// placed where their profile last had them. Returns the live topology as well.
pub(crate) fn plan(
    ports: &Ports,
    patch: &settings_domain::LayoutPatch,
) -> Result<(Topology, ResolvedPatch), AppError> {
    let topology = ports.topology.read()?;
    let profiles = ports.profiles.load()?;
    let active = active_profile(&profiles, &topology);
    let planned = with_saved_positions(&topology, &profiles, active.as_ref());
    let resolved = settings_domain::LayoutPolicy::validate(&planned, patch)?;
    Ok((topology, resolved))
}

/// The profile kanshi currently selects for the connected heads.
pub(crate) fn active_profile(profiles: &ProfileSet, topology: &Topology) -> Option<ProfileName> {
    profiles.match_active(&topology.identities())
}

/// Order for direct compositor writes. Hyprland refuses overlapping outputs, so
/// when the patched output gets smaller (or goes away) it moves first and the
/// outputs that follow it go after; when it grows, the followers make room first.
fn live_order<'a>(topology: &'a Topology, patch: &ResolvedPatch) -> Vec<&'a Output> {
    let patched_first = patch.enable == Some(false)
        || topology.find(&patch.connector).is_some_and(|o| {
            let after = patch.mode.or(o.mode).map(|m| {
                logical_size(
                    m.resolution,
                    patch.scale.unwrap_or(o.scale),
                    patch.transform.unwrap_or(o.transform),
                )
            });
            matches!((o.logical_size(), after), (Some(b), Some(a)) if a.0 <= b.0 && a.1 <= b.1)
        });
    let mut outputs: Vec<&Output> = topology.outputs().iter().collect();
    outputs.sort_by_key(|o| (o.connector == patch.connector) != patched_first);
    outputs
}

/// Persist the patch into the matching kanshi profile, reload, then confirm.
/// Falls back to the compositor when kanshi cannot reload.
pub(crate) fn apply_layout(
    ports: &Ports,
    topology: &Topology,
    patch: &ResolvedPatch,
) -> Result<ApplyOutcome, AppError> {
    let mut profiles = ports.profiles.load()?;
    let ok = match active_profile(&profiles, topology) {
        Some(active) => {
            profiles
                .upsert_layout(&active, topology, Some(patch))
                .stored
        }
        None => profiles.upsert_home_full_snapshot(topology, Some(patch)),
    };
    if !ok {
        return Err(AppError::Rejected(format!(
            "{} cannot be stored in the display profile",
            patch.connector
        )));
    }
    ports.profiles.save(&profiles)?;

    let mut degraded = false;
    if let Err(err) = ports.controller.reload() {
        tracing::warn!(%err, "kanshi reload failed; applying directly");
        if topology.find(&patch.connector).is_none() {
            return Err(AppError::Rejected(format!(
                "{} disappeared",
                patch.connector
            )));
        }
        for output in live_order(topology, patch) {
            let config = config_for(output, Some(patch));
            let unchanged = *output == apply_to_output(output, &config);
            if !unchanged || output.connector == patch.connector {
                ports.live.apply(&config)?;
            }
        }
        degraded = true;
    }

    let mut expectations = vec![Expectation::of_patch(patch)];
    expectations.extend(Expectation::positions_of_patch(patch, topology));
    let verified = verify(ports, &expectations);
    Ok(match (degraded, verified) {
        (true, _) => ApplyOutcome::Degraded,
        (false, true) => ApplyOutcome::Verified,
        (false, false) => ApplyOutcome::Unverified,
    })
}

fn apply_to_output(output: &Output, config: &OutputConfig) -> Output {
    Output {
        mode: config.mode,
        position: config.position,
        scale: config.scale,
        transform: config.transform,
        enabled: config.enabled,
        ..output.clone()
    }
}

/// Put the previous profiles back. `original` (when known) is the topology as it
/// was before the change: it lets the compositor be told directly if kanshi
/// cannot reload, and lets the result be verified for every output.
pub(crate) fn restore_profiles(
    ports: &Ports,
    before: &ProfileSet,
    original: Option<&Topology>,
) -> Result<ApplyOutcome, AppError> {
    ports.profiles.save(before)?;

    let mut degraded = false;
    if let Err(err) = ports.controller.reload() {
        let Some(original) = original else {
            return Err(err.into());
        };
        tracing::warn!(%err, "kanshi reload failed during restore; applying directly");
        let enabled_first = original
            .outputs()
            .iter()
            .filter(|o| o.enabled)
            .chain(original.outputs().iter().filter(|o| !o.enabled));
        for output in enabled_first {
            ports.live.apply(&config_for(output, None))?;
        }
        degraded = true;
    }

    let Some(original) = original else {
        return Ok(ApplyOutcome::Unverified);
    };
    let expectations: Vec<_> = original
        .outputs()
        .iter()
        .map(Expectation::of_output)
        .collect();
    let verified = verify(ports, &expectations);
    Ok(match (degraded, verified) {
        (true, _) => ApplyOutcome::Degraded,
        (false, true) => ApplyOutcome::Verified,
        (false, false) => ApplyOutcome::Unverified,
    })
}
