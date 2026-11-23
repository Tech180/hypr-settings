use settings_domain::arrangement::{Layout, Rect, Tile};
use settings_domain::profile::{Criteria, ProfileOutput};
use settings_domain::{ConnectorName, Output, SCALE_PRESETS, Transform};

use super::apply_layout::{active_profile, with_saved_positions};
use crate::dto::{
    DisplayView, HzChip, OutputView, ProfileOutputView, ProfileView, ResolutionChip, ScaleChip,
    TransformChip, TrialView,
};
use crate::error::AppError;
use crate::ports::Ports;

pub(crate) fn refresh_view(
    ports: &Ports,
    trial: Option<TrialView>,
) -> Result<DisplayView, AppError> {
    let topology = ports.topology.read()?;
    let profiles = ports.profiles.load()?;
    let prefs = ports.prefs.load().unwrap_or_default();
    let active = active_profile(&profiles, &topology);
    let planned = with_saved_positions(&topology, &profiles, active.as_ref());

    let mut outputs: Vec<OutputView> = planned
        .outputs()
        .iter()
        .map(|o| {
            let custom = prefs.custom_hz.get(o.connector.as_str());
            output_view(o, custom.map_or(&[][..], Vec::as_slice))
        })
        .collect();
    outputs.sort_by(|a, b| {
        b.enabled
            .cmp(&a.enabled)
            .then_with(|| a.connector.cmp(&b.connector))
    });

    let profile_views = profiles
        .profiles
        .iter()
        .map(|p| ProfileView {
            name: p.name.to_string(),
            output_count: p.outputs.len(),
            active: active.as_ref() == Some(&p.name),
            removable: !p.name.is_default(),
            outputs: p.outputs.iter().map(profile_output_view).collect(),
        })
        .collect();

    Ok(DisplayView {
        outputs: outputs.clone(),
        profiles: profile_views,
        active_profile: active.map(|n| n.to_string()),
        bar_host: prefs.bar_host_connector.clone().or_else(|| {
            planned
                .bar_host(prefs.bar_host_connector.as_deref())
                .map(|o| o.connector.to_string())
        }),
        trial,
        warnings: layout_warnings(&outputs),
    })
}

fn profile_output_view(slot: &ProfileOutput) -> ProfileOutputView {
    let label = match (&slot.criteria, slot.rest_glob) {
        (Criteria::Any, true) => "Other displays".into(),
        (Criteria::Any, false) => "Any display".into(),
        (Criteria::Named(name), _) => name.clone(),
    };
    let detail = slot
        .mode
        .as_ref()
        .map(|mode| {
            let scale = slot.scale.map(|s| s.label()).unwrap_or_else(|| "1×".into());
            format!(
                "{}x{}@{} · {scale}",
                mode.resolution.width,
                mode.resolution.height,
                mode.refresh.rounded_hz()
            )
        })
        .unwrap_or_default();
    ProfileOutputView {
        label,
        enabled: slot.enable,
        position: slot.position.map(|p| (p.x, p.y)),
        detail,
    }
}

fn layout_warnings(outputs: &[OutputView]) -> Vec<String> {
    let tiles: Vec<Tile> = outputs
        .iter()
        .filter(|o| o.enabled && o.logical_width > 0 && o.logical_height > 0)
        .filter_map(|o| {
            ConnectorName::try_from(o.connector.as_str())
                .ok()
                .map(|connector| Tile {
                    connector,
                    rect: Rect::new(o.x, o.y, o.logical_width, o.logical_height),
                })
        })
        .collect();
    if tiles.len() >= 2 && !Layout::new(tiles).is_connected() {
        vec!["Displays are not connected".into()]
    } else {
        Vec::new()
    }
}

fn output_view(output: &Output, custom_hz: &[u32]) -> OutputView {
    let current = output.mode;
    let (width, height, refresh_hz) = current.map_or((0, 0, 0), |m| {
        (
            m.resolution.width,
            m.resolution.height,
            m.refresh.rounded_hz(),
        )
    });
    let (logical_width, logical_height) = output.logical_size().unwrap_or((0, 0));

    let resolutions = output
        .modes
        .resolutions()
        .into_iter()
        .map(|r| ResolutionChip {
            width: r.width,
            height: r.height,
            active: current.is_some_and(|m| m.resolution == r),
        })
        .collect();

    let hz_options = current
        .map(|m| output.modes.hz_options(m.resolution, refresh_hz, custom_hz))
        .unwrap_or_default()
        .into_iter()
        .map(|hz| HzChip {
            hz,
            active: hz == refresh_hz,
            custom: custom_hz.contains(&hz),
        })
        .collect();

    let mut scales: Vec<_> = SCALE_PRESETS.to_vec();
    if !scales.contains(&output.scale) {
        scales.push(output.scale);
        scales.sort();
    }
    let scale_options = scales
        .into_iter()
        .map(|s| ScaleChip {
            hundredths: s.hundredths(),
            label: s.label(),
            active: s == output.scale,
        })
        .collect();

    let transform_options = Transform::ALL
        .into_iter()
        .map(|t| TransformChip {
            hypr: t.to_hypr(),
            label: t.label(),
            active: t == output.transform,
        })
        .collect();

    OutputView {
        connector: output.connector.to_string(),
        description: output.description.clone(),
        make: output.make.clone(),
        model: output.model.clone(),
        kind: output.kind,
        enabled: output.enabled,
        focused: output.focused,
        ultrawide: output.is_ultrawide(),
        width,
        height,
        refresh_hz,
        x: output.position.x,
        y: output.position.y,
        logical_width,
        logical_height,
        flipped: output.transform.is_flipped(),
        suggested_x: output.position.x,
        suggested_y: output.position.y,
        resolutions,
        hz_options,
        scale_options,
        transform_options,
    }
}
