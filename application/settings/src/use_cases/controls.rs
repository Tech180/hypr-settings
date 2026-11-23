//! Brightness, contrast, overdrive and input source of the connected displays.

use settings_domain::controls::{
    BrightnessSnapshot, BrightnessTarget, Control, ControlKind, DdcDisplay, FeatureProbe,
    InputSources, VCP_BRIGHTNESS, VCP_CONTRAST, clamp_percent, edid_supports_10bit,
    edid_supports_hdr, feature_caps, match_ddc, port_info,
};
use settings_domain::{ConnectorName, DisplayKind, Output, Topology};

use crate::dto::{BrightnessView, OutputControls};
use crate::error::AppError;
use crate::ports::Ports;

/// Brightness is a rounded middle when nothing can be read.
const UNKNOWN_BRIGHTNESS: u8 = 50;

fn locked(reason: &str) -> Control {
    Control {
        kind: ControlKind::None,
        percent: None,
        locked: true,
        reason: reason.to_owned(),
        code: None,
    }
}

fn ddc_for(ports: &Ports, output: &Output) -> Option<DdcDisplay> {
    let displays = ports.ddc.detect(false);
    match_ddc(
        &displays,
        output.connector.as_str(),
        &output.model,
        &output.serial,
    )
    .cloned()
}

fn ddc_control(ports: &Ports, bus: u32, code: u8, missing: &str) -> Control {
    match ports.ddc.read_percent(bus, code) {
        Some(percent) => Control {
            kind: ControlKind::Ddc,
            percent: Some(percent),
            locked: false,
            reason: String::new(),
            code: Some(code),
        },
        None => locked(missing),
    }
}

fn brightness_control(ports: &Ports, output: &Output) -> Control {
    if !output.enabled {
        return locked("Display is off");
    }
    if output.kind == DisplayKind::Laptop
        && let Ok(Some(percent)) = ports.backlight.read()
    {
        return Control {
            kind: ControlKind::Internal,
            percent: Some(percent),
            locked: false,
            reason: String::new(),
            code: None,
        };
    }
    match ddc_for(ports, output) {
        Some(display) => ddc_control(
            ports,
            display.bus,
            VCP_BRIGHTNESS,
            "The display did not answer DDC/CI",
        ),
        None => locked("No backlight or DDC/CI control"),
    }
}

pub(crate) fn output_controls(
    ports: &Ports,
    connector: &ConnectorName,
) -> Result<OutputControls, AppError> {
    let topology = ports.topology.read()?;
    let output = topology
        .find(connector)
        .ok_or_else(|| AppError::Rejected(format!("{connector} is not connected")))?;

    let drm = ports.probe.drm(connector);
    let edid = drm.edid.as_deref().unwrap_or_default();
    let caps = feature_caps(FeatureProbe {
        vrr_capable: drm.vrr_capable,
        max_bpc: drm.max_bpc,
        edid_hdr: edid_supports_hdr(edid),
        edid_10bit: edid_supports_10bit(edid),
        live_hdr: output.hdr,
        live_ten_bit: output.ten_bit,
        live_vrr: output.vrr,
    });

    let ddc = output.enabled.then(|| ddc_for(ports, output)).flatten();
    let no_ddc = if output.enabled {
        "DDC/CI is not available"
    } else {
        "Display is off"
    };
    let (contrast, overdrive, input) = match &ddc {
        Some(display) => {
            let overdrive = ports.ddc.overdrive_code(display.bus).map_or_else(
                || locked("No overdrive control"),
                |code| {
                    ddc_control(
                        ports,
                        display.bus,
                        code,
                        "The display did not answer DDC/CI",
                    )
                },
            );
            (
                ddc_control(
                    ports,
                    display.bus,
                    VCP_CONTRAST,
                    "The display did not answer DDC/CI",
                ),
                overdrive,
                ports.ddc.input_sources(display.bus),
            )
        }
        None => (locked(no_ddc), locked(no_ddc), InputSources::default()),
    };
    let sink_input = input.current.as_ref().map(|i| i.label.clone());

    Ok(OutputControls {
        connector: connector.to_string(),
        brightness: brightness_control(ports, output),
        contrast,
        overdrive,
        port: Some(port_info(connector.as_str(), &drm, sink_input)),
        input,
        caps,
    })
}

/// Where a brightness change for `output` goes, without reading anything back.
fn target_of(ports: &Ports, output: &Output) -> Option<BrightnessTarget> {
    if !output.enabled {
        return None;
    }
    if output.kind == DisplayKind::Laptop && matches!(ports.backlight.read(), Ok(Some(_))) {
        return Some(BrightnessTarget::Internal);
    }
    ddc_for(ports, output).map(|d| BrightnessTarget::Ddc { bus: d.bus })
}

struct Targets {
    all: Vec<BrightnessTarget>,
    host: Option<BrightnessTarget>,
}

fn targets(ports: &Ports, topology: &Topology) -> Targets {
    let mut all = Vec::new();
    for output in topology.outputs() {
        if let Some(target) = target_of(ports, output)
            && !all.contains(&target)
        {
            all.push(target);
        }
    }
    let preferred = ports.prefs.load().unwrap_or_default().bar_host_connector;
    let host = topology
        .bar_host(preferred.as_deref())
        .and_then(|o| target_of(ports, o))
        .or_else(|| all.first().copied());
    Targets { all, host }
}

fn read_target(ports: &Ports, target: BrightnessTarget) -> Option<u8> {
    match target {
        BrightnessTarget::Internal => ports.backlight.read().ok().flatten(),
        BrightnessTarget::Ddc { bus } => ports.ddc.read_percent(bus, VCP_BRIGHTNESS),
    }
}

fn write_target(ports: &Ports, target: BrightnessTarget, percent: u8) -> Result<(), AppError> {
    match target {
        BrightnessTarget::Internal => ports.backlight.set(percent)?,
        BrightnessTarget::Ddc { bus } => ports.ddc.write_percent(bus, VCP_BRIGHTNESS, percent)?,
    }
    Ok(())
}

/// The targets for the displays connected right now, with `percent` as the shared value.
fn fresh_snapshot(ports: &Ports, percent: Option<u8>) -> Result<BrightnessSnapshot, AppError> {
    let topology = ports.topology.read()?;
    let found = targets(ports, &topology);
    let sync = ports.prefs.load().unwrap_or_default().sync_brightness;
    let percent = percent
        .or_else(|| found.host.and_then(|t| read_target(ports, t)))
        .or_else(|| ports.brightness.read().ok().flatten().map(|s| s.percent))
        .unwrap_or(UNKNOWN_BRIGHTNESS);
    Ok(BrightnessSnapshot {
        percent,
        sync,
        targets: found.all,
        host: found.host,
    })
}

fn publish(ports: &Ports, snapshot: &BrightnessSnapshot) {
    if let Err(err) = ports.brightness.write(snapshot) {
        tracing::warn!(%err, "could not share the brightness targets");
    }
}

/// What the displays are doing now; also refreshes the targets the bar and keys use.
pub(crate) fn brightness(ports: &Ports) -> Result<BrightnessView, AppError> {
    let topology = ports.topology.read()?;
    let found = targets(ports, &topology);
    let sync = ports.prefs.load().unwrap_or_default().sync_brightness;
    let percent = found.host.and_then(|t| read_target(ports, t));
    publish(
        ports,
        &BrightnessSnapshot {
            percent: percent
                .or_else(|| ports.brightness.read().ok().flatten().map(|s| s.percent))
                .unwrap_or(UNKNOWN_BRIGHTNESS),
            sync,
            targets: found.all.clone(),
            host: found.host,
        },
    );
    Ok(BrightnessView {
        percent,
        sync,
        targets: found.all.len(),
    })
}

/// Looks for displays again (a plug or unplug) before reading.
pub(crate) fn refresh_brightness(ports: &Ports) -> Result<BrightnessView, AppError> {
    ports.ddc.detect(true);
    brightness(ports)
}

/// Writes `percent` to the displays the snapshot names: all of them when synchronised,
/// otherwise the host. Works from the shared file, so the brightness keys do not have to
/// detect displays again.
fn apply(ports: &Ports, snapshot: &BrightnessSnapshot, percent: u8) -> Result<(), AppError> {
    let chosen: Vec<BrightnessTarget> = if snapshot.sync {
        snapshot.targets.clone()
    } else {
        snapshot
            .host
            .or_else(|| snapshot.targets.first().copied())
            .into_iter()
            .collect()
    };
    if chosen.is_empty() {
        return Err(AppError::Rejected(
            "no display can change its brightness".into(),
        ));
    }
    let mut written = 0;
    let mut last_error = None;
    for target in chosen {
        match write_target(ports, target, percent) {
            Ok(()) => written += 1,
            Err(err) => {
                tracing::warn!(%err, ?target, "brightness write failed");
                last_error = Some(err);
            }
        }
    }
    publish(
        ports,
        &BrightnessSnapshot {
            percent,
            ..snapshot.clone()
        },
    );
    match (written, last_error) {
        (0, Some(err)) => Err(err),
        _ => Ok(()),
    }
}

fn snapshot_or_fresh(ports: &Ports) -> Result<BrightnessSnapshot, AppError> {
    match ports.brightness.read() {
        Ok(Some(snapshot)) if !snapshot.targets.is_empty() => Ok(snapshot),
        _ => fresh_snapshot(ports, None),
    }
}

pub(crate) fn set_brightness(ports: &Ports, percent: u8) -> Result<(), AppError> {
    let snapshot = snapshot_or_fresh(ports)?;
    apply(ports, &snapshot, clamp_percent(i64::from(percent)))
}

pub(crate) fn adjust_brightness(ports: &Ports, delta: i32) -> Result<u8, AppError> {
    let snapshot = snapshot_or_fresh(ports)?;
    let next = clamp_percent(i64::from(snapshot.percent) + i64::from(delta));
    apply(ports, &snapshot, next)?;
    Ok(next)
}

pub(crate) fn set_sync(ports: &Ports, sync: bool) -> Result<(), AppError> {
    let mut prefs = ports.prefs.load().unwrap_or_default();
    prefs.sync_brightness = sync;
    ports.prefs.save(&prefs)?;
    if let Ok(Some(mut snapshot)) = ports.brightness.read() {
        snapshot.sync = sync;
        publish(ports, &snapshot);
    }
    Ok(())
}

/// The bar moved: the shared file has to name the new host.
pub(crate) fn refresh_host(ports: &Ports) {
    let Ok(Some(old)) = ports.brightness.read() else {
        return;
    };
    match fresh_snapshot(ports, Some(old.percent)) {
        Ok(snapshot) => publish(ports, &snapshot),
        Err(err) => tracing::warn!(%err, "could not update the brightness host"),
    }
}

fn ddc_bus(ports: &Ports, connector: &ConnectorName) -> Result<u32, AppError> {
    let topology = ports.topology.read()?;
    let output = topology
        .find(connector)
        .filter(|o| o.enabled)
        .ok_or_else(|| AppError::Rejected(format!("{connector} is off")))?;
    ddc_for(ports, output)
        .map(|d| d.bus)
        .ok_or_else(|| AppError::Rejected(format!("{connector} has no DDC/CI control")))
}

pub(crate) fn set_contrast(
    ports: &Ports,
    connector: &ConnectorName,
    percent: u8,
) -> Result<(), AppError> {
    let bus = ddc_bus(ports, connector)?;
    ports
        .ddc
        .write_percent(bus, VCP_CONTRAST, clamp_percent(i64::from(percent)))?;
    Ok(())
}

pub(crate) fn set_overdrive(
    ports: &Ports,
    connector: &ConnectorName,
    percent: u8,
) -> Result<(), AppError> {
    let bus = ddc_bus(ports, connector)?;
    let code = ports
        .ddc
        .overdrive_code(bus)
        .ok_or_else(|| AppError::Rejected(format!("{connector} has no overdrive control")))?;
    ports
        .ddc
        .write_percent(bus, code, clamp_percent(i64::from(percent)))?;
    Ok(())
}

pub(crate) fn set_input_source(
    ports: &Ports,
    connector: &ConnectorName,
    code: u8,
) -> Result<(), AppError> {
    let bus = ddc_bus(ports, connector)?;
    ports.ddc.set_input_source(bus, code)?;
    Ok(())
}
