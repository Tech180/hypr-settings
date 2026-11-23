//! Props of the rows that talk to the hardware: main display, custom refresh rates, HDR /
//! 10-bit / Adaptive Sync, DDC/CI controls, brightness and the night light.

use std::rc::Rc;

use settings_application::dto::{BrightnessView, NightLightView, OutputControls, OutputView};
use settings_domain::controls::{Capability, Control, Flag, port_line, port_tooltip};
use settings_domain::night_light::{KELVIN_MAX, KELVIN_MIN};

use crate::features::display::components::feature_grid::{FeatureGridProps, FeatureTileProps};
use crate::features::display::components::hz_group::{HzGroupProps, HzRateOption};
use crate::features::display::interfaces::DisplayActions;
use crate::shared::icon::Icon;
use crate::shared::section_header::HeaderTag;
use crate::shared::settings_group::{RowControl, SettingRowProps, SettingsGroupProps, SliderProps};

const CHECKING: &str = "Checking the display…";

/// What the slow hardware probes have found so far; `None` while they are still running.
#[derive(Clone, Copy, Default)]
pub struct Hardware<'a> {
    pub controls: Option<&'a OutputControls>,
    pub brightness: Option<&'a BrightnessView>,
    pub night: Option<&'a NightLightView>,
}

fn row(
    title: &str,
    subtitle: Option<String>,
    sensitive: bool,
    control: RowControl,
) -> SettingRowProps {
    SettingRowProps {
        title: title.into(),
        subtitle,
        sensitive,
        control,
        leading: None,
        badge: None,
    }
}

fn percent_slider(value: u8, on_change: Rc<dyn Fn(f64)>) -> RowControl {
    RowControl::Slider(SliderProps {
        value: f64::from(value),
        min: 0.0,
        max: 100.0,
        step: 1.0,
        unit: "%",
        on_change,
    })
}

/// The row of a DDC/CI level: a live slider, or a dimmed one that says what is missing.
fn level_row(
    title: &str,
    subtitle: &str,
    control: Option<&Control>,
    on_change: &Rc<dyn Fn(f64)>,
) -> SettingRowProps {
    match control {
        None => row(
            title,
            Some(CHECKING.into()),
            false,
            percent_slider(0, Rc::clone(on_change)),
        ),
        Some(c) => match c.percent.filter(|_| !c.locked) {
            Some(percent) => row(
                title,
                Some(subtitle.into()),
                true,
                percent_slider(percent, Rc::clone(on_change)),
            ),
            None => row(
                title,
                Some(c.reason.clone()),
                false,
                percent_slider(0, Rc::clone(on_change)),
            ),
        },
    }
}

fn feature_tile(
    flag: Flag,
    title: &'static str,
    subtitle: &'static str,
    icon: Icon,
    capability: Option<&Capability>,
    actions: &DisplayActions,
) -> FeatureTileProps {
    let set_flag = Rc::clone(&actions.set_flag);
    let (active, supported, reason) = match capability {
        Some(c) => (c.enabled, c.supported, c.reason.clone()),
        None => (false, false, CHECKING.to_owned()),
    };
    FeatureTileProps {
        title,
        subtitle,
        icon,
        active,
        sensitive: supported,
        tooltip: if supported { None } else { Some(reason) },
        on_toggle: Rc::new(move |value| {
            if value != active {
                set_flag(flag, value);
            }
        }),
    }
}

fn main_display_row(
    output: &OutputView,
    bar_host: Option<&str>,
    actions: &DisplayActions,
) -> SettingRowProps {
    let is_main = bar_host == Some(output.connector.as_str());
    let set_main = Rc::clone(&actions.set_main);
    row(
        "Main display",
        Some("Hosts the bar and the quick controls".into()),
        output.enabled && !is_main,
        RowControl::Button {
            label: if is_main {
                "Main display"
            } else {
                "Set as main"
            }
            .into(),
            on_click: Rc::new(move || set_main()),
        },
    )
}

pub fn hz_group(output: &OutputView, actions: &DisplayActions) -> HzGroupProps {
    HzGroupProps {
        enabled: output.enabled,
        current_hz: output.refresh_hz,
        options: output
            .hz_options
            .iter()
            .map(|c| HzRateOption {
                hz: c.hz,
                active: c.active,
                custom: c.custom,
            })
            .collect(),
        on_pick: Rc::clone(&actions.set_refresh),
        on_add_custom: Rc::clone(&actions.add_custom_hz),
        on_remove_custom: Rc::clone(&actions.remove_custom_hz),
    }
}

pub fn feature_grid(
    output: &OutputView,
    hardware: Hardware<'_>,
    actions: &DisplayActions,
) -> FeatureGridProps {
    let controls = hardware
        .controls
        .filter(|c| c.connector == output.connector);
    let caps = controls.map(|c| &c.caps);
    FeatureGridProps {
        tiles: vec![
            feature_tile(
                Flag::Hdr,
                "HDR",
                "High dynamic range",
                Icon::Sun,
                caps.map(|c| &c.hdr),
                actions,
            ),
            feature_tile(
                Flag::TenBit,
                "10-bit",
                "Deep color",
                Icon::Droplet,
                caps.map(|c| &c.ten_bit),
                actions,
            ),
            feature_tile(
                Flag::Vrr,
                "Adaptive Sync",
                "Match refresh to frame rate",
                Icon::RefreshCw,
                caps.map(|c| &c.vrr),
                actions,
            ),
        ],
    }
}

pub fn main_display_group(
    output: &OutputView,
    bar_host: Option<&str>,
    actions: &DisplayActions,
) -> SettingsGroupProps {
    SettingsGroupProps {
        rows: vec![main_display_row(output, bar_host, actions)],
    }
}

/// Contrast, overdrive, and input source rows for the selected output.
pub fn ddc_group(
    output: &OutputView,
    hardware: Hardware<'_>,
    actions: &DisplayActions,
) -> SettingsGroupProps {
    let controls = hardware
        .controls
        .filter(|c| c.connector == output.connector);
    let mut rows = vec![level_row(
        "Contrast",
        "Set on the display over DDC/CI",
        controls.map(|c| &c.contrast),
        &actions.set_contrast,
    )];
    let has_overdrive =
        controls.is_none_or(|c| c.overdrive.percent.is_some() || !c.overdrive.locked);
    if has_overdrive {
        rows.push(level_row(
            "Overdrive",
            "Pixel response, set over DDC/CI",
            controls.map(|c| &c.overdrive),
            &actions.set_overdrive,
        ));
    }
    if let Some(c) = controls
        && c.input.options.len() > 1
    {
        let codes: Vec<u8> = c.input.options.iter().map(|o| o.code).collect();
        let current = c
            .input
            .current
            .as_ref()
            .and_then(|cur| codes.iter().position(|code| *code == cur.code));
        let set_input = Rc::clone(&actions.set_input);
        rows.push(row(
            "Input source",
            Some("Switching away may blank this picture".into()),
            true,
            RowControl::Select {
                options: c.input.options.iter().map(|o| o.label.clone()).collect(),
                selected: current.unwrap_or(0),
                on_select: Rc::new(move |index| {
                    if index != current.unwrap_or(usize::MAX)
                        && let Some(&code) = codes.get(index)
                    {
                        set_input(code);
                    }
                }),
            },
        ));
    }
    SettingsGroupProps { rows }
}

/// Brightness and night light inside the left bento column.
pub fn global_env_group(hardware: Hardware<'_>, actions: &DisplayActions) -> SettingsGroupProps {
    let mut rows = vec![row(
        "Global Environment",
        Some("Applies to all displays".into()),
        true,
        RowControl::Section,
    )];
    match hardware.brightness {
        None => rows.push(row(
            "Brightness",
            Some(CHECKING.into()),
            false,
            percent_slider(0, Rc::clone(&actions.set_brightness)),
        )),
        Some(view) => {
            let adjustable = view.targets > 0;
            rows.push(row(
                "Brightness",
                Some(match (view.percent, adjustable) {
                    (_, false) => "No display can change its brightness".to_owned(),
                    (_, true) if view.sync => format!("Applies to {} display(s)", view.targets),
                    (_, true) => "Applies to the main display".to_owned(),
                }),
                adjustable,
                percent_slider(
                    view.percent.unwrap_or(50),
                    Rc::clone(&actions.set_brightness),
                ),
            ));
            let set_sync = Rc::clone(&actions.set_sync);
            let current = view.sync;
            rows.push(row(
                "Sync displays",
                Some("Move universally".into()),
                view.targets > 1,
                RowControl::Switch {
                    active: current,
                    on_toggle: Rc::new(move |value| {
                        if value != current {
                            set_sync(value);
                        }
                    }),
                },
            ));
        }
    }
    let (on, kelvin) = hardware.night.map_or((false, 4000), |n| (n.on, n.kelvin));
    let known = hardware.night.is_some();
    let set_night = Rc::clone(&actions.set_night);
    rows.push(row(
        "Night light",
        Some(if on {
            "Fixed temperature; the schedule is paused".to_owned()
        } else {
            "Following schedule".to_owned()
        }),
        known,
        RowControl::Switch {
            active: on,
            on_toggle: Rc::new(move |value| {
                if value != on {
                    set_night(value);
                }
            }),
        },
    ));
    if on {
        rows.push(row(
            "Color temperature",
            Some("Warmer on the left".into()),
            known,
            RowControl::Slider(SliderProps {
                value: f64::from(kelvin),
                min: f64::from(KELVIN_MIN),
                max: f64::from(KELVIN_MAX),
                step: 50.0,
                unit: " K",
                on_change: Rc::clone(&actions.set_kelvin),
            }),
        ));
    }
    SettingsGroupProps { rows }
}

/// `1.4 · HBR3 ×4` next to the connector, with the host and sink details on hover.
pub fn port_tag(hardware: Hardware<'_>, connector: &str) -> Option<HeaderTag> {
    let controls = hardware.controls.filter(|c| c.connector == connector)?;
    let port = controls.port.as_ref()?;
    Some(HeaderTag {
        text: format!("{} {}", port.standard, port_line(port)),
        tooltip: port_tooltip(port),
    })
}
