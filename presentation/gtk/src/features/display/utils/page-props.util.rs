//! Maps the application's `DisplayView` onto the props of every display component.

use std::rc::Rc;

use settings_application::drag;
use settings_application::dto::{DisplayView, OutputView, ProfileView};
use settings_domain::Transform;

use super::format_mode::{coordinates, format_mode, kind_label, resolution_label};
use super::hardware_props::{self, Hardware};
use crate::features::display::components::arrangement_canvas::{
    ArrangementCanvasProps, PreviewDrop, TileProps,
};
use crate::features::display::components::display_dashboard::{
    DashboardBlock, DashboardColumnProps, DisplayDashboardProps,
};
use crate::features::display::components::profile_detail::{ProfileDetailProps, SavedOutputProps};
use crate::features::display::components::profiles_group::{ProfileRowProps, ProfilesGroupProps};
use crate::features::display::interfaces::{DisplayActions, DisplayPageProps};
use crate::shared::icon::Icon;
use crate::shared::section_header::{HeadingSize, SectionHeaderProps};
use crate::shared::segmented::SegmentedProps;
use crate::shared::settings_group::{RowControl, SettingRowProps, SettingsGroupProps};

/// Size of a tile for a display that has no live mode, such as one that is switched off.
const FALLBACK_SIZE: (i32, i32) = (1920, 1080);

/// Keeps the current selection while it still exists, else the focused output, else the first.
pub fn resolve_selection(view: &DisplayView, current: Option<&str>) -> Option<String> {
    current
        .filter(|c| view.outputs.iter().any(|o| o.connector == *c))
        .map(str::to_owned)
        .or_else(|| {
            view.outputs
                .iter()
                .find(|o| o.focused)
                .or(view.outputs.first())
                .map(|o| o.connector.clone())
        })
}

pub fn page_props(
    view: &DisplayView,
    selected: &str,
    hardware: Hardware<'_>,
    actions: &DisplayActions,
) -> Option<DisplayPageProps> {
    let output = view.outputs.iter().find(|o| o.connector == selected)?;
    Some(DisplayPageProps {
        arrangement: arrangement(view, selected, actions),
        notice: view.warnings.first().cloned(),
        output_header: output_header(output, hardware),
        dashboard: dashboard(output, view.bar_host.as_deref(), hardware, actions),
        profiles: profiles(view, actions),
    })
}

pub fn arrangement_header() -> SectionHeaderProps {
    SectionHeaderProps {
        title: "Arrangement".into(),
        subtitle: Some("Drag to rearrange. Snaps to align edges.".into()),
        badge: None,
        tag: None,
        size: HeadingSize::Large,
    }
}

pub fn profile_detail_props(
    view: &DisplayView,
    name: &str,
    actions: &DisplayActions,
) -> Option<crate::features::display::components::profile_detail::ProfileDetailProps> {
    let profile = view.profiles.iter().find(|p| p.name == name)?;
    Some(ProfileDetailProps {
        name: profile.name.clone(),
        subtitle: match profile.output_count {
            1 => "1 display configured in this arrangement".to_owned(),
            n => format!("{n} displays configured in this arrangement"),
        },
        outputs: profile
            .outputs
            .iter()
            .map(|o| SavedOutputProps {
                title: o.label.clone(),
                subtitle: saved_subtitle(o.enabled, &o.detail),
                badge: o.position.map(|(x, y)| coordinates(x, y)),
            })
            .collect(),
        removable: profile.removable,
        on_back: Rc::clone(&actions.close_profile),
        on_delete: {
            let (delete, name) = (Rc::clone(&actions.delete_profile), profile.name.clone());
            Rc::new(move || delete(name.clone()))
        },
        on_apply: {
            let (apply, name) = (Rc::clone(&actions.apply_profile), profile.name.clone());
            Rc::new(move || apply(name.clone()))
        },
    })
}

fn saved_subtitle(enabled: Option<bool>, detail: &str) -> String {
    match (enabled, detail.is_empty()) {
        (Some(false), true) => "Disabled".to_owned(),
        (Some(false), false) => format!("Disabled • {detail}"),
        _ => detail.to_owned(),
    }
}

/// A click on an already active choice does nothing; any other becomes `on_pick(value)`.
fn pick<T: Copy + 'static>(values: Vec<(T, bool)>, on_pick: Rc<dyn Fn(T)>) -> Rc<dyn Fn(usize)> {
    Rc::new(move |index| {
        if let Some(&(value, false)) = values.get(index) {
            on_pick(value);
        }
    })
}

fn tile_size(output: &OutputView) -> (i32, i32) {
    if output.logical_width > 0 && output.logical_height > 0 {
        return (output.logical_width, output.logical_height);
    }
    output
        .resolutions
        .first()
        .and_then(|r| Some((i32::try_from(r.width).ok()?, i32::try_from(r.height).ok()?)))
        .unwrap_or(FALLBACK_SIZE)
}

fn arrangement(
    view: &DisplayView,
    selected: &str,
    actions: &DisplayActions,
) -> ArrangementCanvasProps {
    let outputs = Rc::new(view.outputs.clone());
    let preview: PreviewDrop = Rc::new(move |connector, wanted, threshold| {
        drag::preview(&outputs, connector, wanted, threshold)
    });
    ArrangementCanvasProps {
        tiles: view
            .outputs
            .iter()
            .map(|o| {
                let (width, height) = tile_size(o);
                let (x, y) = if o.enabled {
                    (o.x, o.y)
                } else {
                    (o.suggested_x, o.suggested_y)
                };
                TileProps {
                    connector: o.connector.clone(),
                    size_label: if o.width > 0 {
                        resolution_label(o.width, o.height)
                    } else {
                        resolution_label(width as u32, height as u32)
                    },
                    x,
                    y,
                    width,
                    height,
                    enabled: o.enabled,
                    focused: o.focused,
                    selected: o.connector == selected,
                }
            })
            .collect(),
        on_select: Rc::clone(&actions.select_output),
        on_drop: Rc::clone(&actions.move_output),
        preview,
    }
}

fn output_header(output: &OutputView, hardware: Hardware<'_>) -> SectionHeaderProps {
    let status = if output.enabled {
        format!(
            "{} • {}",
            format_mode(output.width, output.height, output.refresh_hz),
            coordinates(output.x, output.y)
        )
    } else {
        "Disabled".to_owned()
    };
    let mut parts = vec![kind_label(output.kind).to_owned()];
    if output.focused {
        parts.push("Focused".into());
    }
    if output.ultrawide {
        parts.push("Ultrawide".into());
    }
    parts.push(status);
    SectionHeaderProps {
        title: if output.description.is_empty() {
            output.connector.clone()
        } else {
            output.description.clone()
        },
        subtitle: Some(parts.join(" • ")),
        badge: Some(output.connector.clone()),
        tag: hardware_props::port_tag(hardware, &output.connector),
        size: HeadingSize::Large,
    }
}

fn segmented<T: Copy + 'static>(
    items: Vec<(String, T, bool)>,
    on_pick: Rc<dyn Fn(T)>,
) -> RowControl {
    RowControl::Segmented(SegmentedProps {
        labels: items.iter().map(|(label, ..)| label.clone()).collect(),
        selected: items.iter().position(|(_, _, active)| *active),
        on_select: pick(items.iter().map(|(_, v, a)| (*v, *a)).collect(), on_pick),
    })
}

fn dashboard(
    output: &OutputView,
    bar_host: Option<&str>,
    hardware: Hardware<'_>,
    actions: &DisplayActions,
) -> DisplayDashboardProps {
    let mut right_blocks = Vec::new();
    if !output.hz_options.is_empty() {
        right_blocks.push(DashboardBlock::Hz(hardware_props::hz_group(
            output, actions,
        )));
    }
    if output.enabled {
        right_blocks.push(DashboardBlock::Features(hardware_props::feature_grid(
            output, hardware, actions,
        )));
    }
    right_blocks.push(DashboardBlock::Group(hardware_props::main_display_group(
        output, bar_host, actions,
    )));
    let ddc = hardware_props::ddc_group(output, hardware, actions);
    if !ddc.rows.is_empty() {
        right_blocks.push(DashboardBlock::Group(ddc));
    }

    DisplayDashboardProps {
        left: DashboardColumnProps {
            blocks: vec![
                DashboardBlock::Group(mode_settings(output, actions)),
                DashboardBlock::Group(hardware_props::global_env_group(hardware, actions)),
            ],
        },
        right: DashboardColumnProps {
            blocks: right_blocks,
        },
    }
}

fn mode_settings(output: &OutputView, actions: &DisplayActions) -> SettingsGroupProps {
    let mut rows = vec![SettingRowProps {
        title: "Use this display".into(),
        sensitive: true,
        subtitle: Some(format!("Power on and output to {}", output.connector)),
        control: enable_control(output, actions),
        leading: None,
        badge: None,
    }];

    if !output.resolutions.is_empty() {
        let set_resolution = Rc::clone(&actions.set_resolution);
        let on_pick: Rc<dyn Fn((u32, u32))> = Rc::new(move |(w, h)| set_resolution(w, h));
        rows.push(SettingRowProps {
            title: "Resolution".into(),
            sensitive: true,
            subtitle: None,
            control: RowControl::Select {
                options: output
                    .resolutions
                    .iter()
                    .map(|c| resolution_label(c.width, c.height))
                    .collect(),
                selected: output
                    .resolutions
                    .iter()
                    .position(|c| c.active)
                    .unwrap_or(0),
                on_select: pick(
                    output
                        .resolutions
                        .iter()
                        .map(|c| ((c.width, c.height), c.active))
                        .collect(),
                    on_pick,
                ),
            },
            leading: None,
            badge: None,
        });
    }
    if !output.scale_options.is_empty() {
        rows.push(SettingRowProps {
            title: "Scale".into(),
            sensitive: true,
            subtitle: None,
            control: segmented(
                output
                    .scale_options
                    .iter()
                    .map(|c| (format!("{}%", c.hundredths), c.hundredths, c.active))
                    .collect(),
                Rc::clone(&actions.set_scale),
            ),
            leading: None,
            badge: None,
        });
    }
    rows.push(SettingRowProps {
        title: "Rotation".into(),
        sensitive: true,
        subtitle: None,
        control: segmented(
            output
                .transform_options
                .iter()
                .map(|c| (c.label.to_owned(), c.hypr, c.active))
                .collect(),
            Rc::clone(&actions.set_transform),
        ),
        leading: None,
        badge: None,
    });
    rows.push(SettingRowProps {
        title: "Flipped".into(),
        sensitive: true,
        subtitle: Some("Mirror the picture horizontally".into()),
        control: flip_control(output, actions),
        leading: None,
        badge: None,
    });
    SettingsGroupProps { rows }
}

fn enable_control(output: &OutputView, actions: &DisplayActions) -> RowControl {
    let set_enabled = Rc::clone(&actions.set_enabled);
    let current = output.enabled;
    RowControl::Switch {
        active: current,
        on_toggle: Rc::new(move |value| {
            if value != current {
                set_enabled(value);
            }
        }),
    }
}

/// Mirroring keeps the rotation: it flips the mirror bit of the active rotation chip.
fn flip_control(output: &OutputView, actions: &DisplayActions) -> RowControl {
    let set_transform = Rc::clone(&actions.set_transform);
    let current = output.flipped;
    let active = output
        .transform_options
        .iter()
        .find(|c| c.active)
        .map_or(0, |c| c.hypr);
    RowControl::Switch {
        active: current,
        on_toggle: Rc::new(move |value| {
            if value != current {
                set_transform(Transform::from_hypr(active).with_flipped(value).to_hypr());
            }
        }),
    }
}

fn profile_icon(name: &str) -> Icon {
    match name {
        "home" => Icon::House,
        "portable" => Icon::Laptop,
        "docked" | "default" => Icon::Monitor,
        _ => Icon::Cpu,
    }
}

fn profiles(view: &DisplayView, actions: &DisplayActions) -> ProfilesGroupProps {
    let names: Vec<String> = view.profiles.iter().map(|p| p.name.clone()).collect();
    let open = Rc::clone(&actions.open_profile);
    ProfilesGroupProps {
        rows: view.profiles.iter().map(profile_row).collect(),
        on_open: Rc::new(move |index| {
            if let Some(name) = names.get(index) {
                open(name.clone());
            }
        }),
        on_save_as: Rc::clone(&actions.save_profile_as),
        active_name: view.active_profile.clone(),
        on_save_active: Rc::clone(&actions.save_active),
    }
}

fn profile_row(profile: &ProfileView) -> ProfileRowProps {
    ProfileRowProps {
        name: profile.name.clone(),
        subtitle: match profile.output_count {
            1 => "1 configured output".to_owned(),
            n => format!("{n} configured outputs"),
        },
        active: profile.active,
        icon: profile_icon(&profile.name),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use settings_application::dto::{
        BrightnessView, HzChip, NightLightView, OutputControls, ProfileOutputView, ResolutionChip,
        ScaleChip, TransformChip,
    };
    use settings_domain::DisplayKind;

    use super::*;

    fn output(connector: &str, focused: bool, enabled: bool) -> OutputView {
        OutputView {
            connector: connector.into(),
            description: format!("{connector} panel"),
            make: String::new(),
            model: String::new(),
            kind: DisplayKind::Monitor,
            enabled,
            focused,
            ultrawide: false,
            width: 1920,
            height: 1080,
            refresh_hz: 60,
            x: 0,
            y: 0,
            logical_width: 1920,
            logical_height: 1080,
            flipped: false,
            suggested_x: 1920,
            suggested_y: 0,
            resolutions: vec![
                ResolutionChip {
                    width: 1920,
                    height: 1080,
                    active: true,
                },
                ResolutionChip {
                    width: 1280,
                    height: 720,
                    active: false,
                },
            ],
            hz_options: vec![
                HzChip {
                    hz: 60,
                    active: true,
                    custom: false,
                },
                HzChip {
                    hz: 144,
                    active: false,
                    custom: false,
                },
            ],
            scale_options: vec![
                ScaleChip {
                    hundredths: 100,
                    label: "1×".into(),
                    active: true,
                },
                ScaleChip {
                    hundredths: 150,
                    label: "1.5×".into(),
                    active: false,
                },
            ],
            transform_options: vec![
                TransformChip {
                    hypr: 0,
                    label: "0°",
                    active: true,
                },
                TransformChip {
                    hypr: 1,
                    label: "90°",
                    active: false,
                },
            ],
        }
    }

    fn view() -> DisplayView {
        DisplayView {
            outputs: vec![output("DP-3", false, true), output("DP-6", true, true)],
            profiles: vec![
                ProfileView {
                    name: "home".into(),
                    output_count: 2,
                    active: true,
                    removable: true,
                    outputs: vec![ProfileOutputView {
                        label: "eDP-1".into(),
                        enabled: Some(true),
                        position: Some((1080, 0)),
                        detail: "1920x1080@60 · 1×".into(),
                    }],
                },
                ProfileView {
                    name: "default".into(),
                    output_count: 1,
                    active: false,
                    removable: false,
                    outputs: Vec::new(),
                },
            ],
            active_profile: Some("home".into()),
            trial: None,
            warnings: Vec::new(),
            bar_host: None,
        }
    }

    #[derive(Default)]
    struct Log(RefCell<Vec<String>>);

    fn actions(log: &Rc<Log>) -> DisplayActions {
        fn logged<A: 'static>(
            log: &Rc<Log>,
            describe: impl Fn(A) -> String + 'static,
        ) -> Rc<dyn Fn(A)> {
            let log = Rc::clone(log);
            Rc::new(move |a| log.0.borrow_mut().push(describe(a)))
        }
        fn logged_unit(log: &Rc<Log>, what: &'static str) -> Rc<dyn Fn()> {
            let log = Rc::clone(log);
            Rc::new(move || log.0.borrow_mut().push(what.into()))
        }
        let l = Rc::clone(log);
        let move_output: Rc<dyn Fn(String, i32, i32)> =
            Rc::new(move |c, x, y| l.0.borrow_mut().push(format!("move {c} {x},{y}")));
        let l = Rc::clone(log);
        let set_resolution: Rc<dyn Fn(u32, u32)> =
            Rc::new(move |w, h| l.0.borrow_mut().push(format!("res {w}x{h}")));
        let l = Rc::clone(log);
        let close_profile: Rc<dyn Fn()> = Rc::new(move || l.0.borrow_mut().push("close".into()));
        DisplayActions {
            select_output: logged(log, |c: String| format!("select {c}")),
            move_output,
            set_resolution,
            set_refresh: logged(log, |hz: u32| format!("hz {hz}")),
            set_scale: logged(log, |s: u16| format!("scale {s}")),
            set_transform: logged(log, |t: i32| format!("rot {t}")),
            set_enabled: logged(log, |e: bool| format!("enable {e}")),
            open_profile: logged(log, |n: String| format!("open {n}")),
            close_profile,
            apply_profile: logged(log, |n: String| format!("apply {n}")),
            delete_profile: logged(log, |n: String| format!("delete {n}")),
            save_profile_as: logged(log, |n: String| format!("save {n}")),
            save_active: logged_unit(log, "save active"),
            set_main: logged_unit(log, "main"),
            add_custom_hz: logged(log, |hz: u32| format!("add hz {hz}")),
            remove_custom_hz: logged(log, |hz: u32| format!("remove hz {hz}")),
            set_brightness: logged(log, |v: f64| format!("brightness {v}")),
            set_sync: logged(log, |on: bool| format!("sync {on}")),
            set_contrast: logged(log, |v: f64| format!("contrast {v}")),
            set_overdrive: logged(log, |v: f64| format!("overdrive {v}")),
            set_input: logged(log, |c: u8| format!("input {c}")),
            set_flag: {
                let l = Rc::clone(log);
                Rc::new(move |flag, on| l.0.borrow_mut().push(format!("flag {flag:?} {on}")))
            },
            set_night: logged(log, |on: bool| format!("night {on}")),
            set_kelvin: logged(log, |v: f64| format!("kelvin {v}")),
        }
    }

    fn mode_row<'a>(props: &'a DisplayPageProps, title: &str) -> &'a SettingRowProps {
        let DashboardBlock::Group(group) = &props.dashboard.left.blocks[0] else {
            panic!("mode group missing");
        };
        group
            .rows
            .iter()
            .find(|r| r.title == title)
            .unwrap_or_else(|| panic!("mode row {title}"))
    }

    fn right_row<'a>(props: &'a DisplayPageProps, title: &str) -> &'a SettingRowProps {
        for block in &props.dashboard.right.blocks {
            if let DashboardBlock::Group(group) = block
                && let Some(row) = group.rows.iter().find(|r| r.title == title)
            {
                return row;
            }
        }
        panic!("right row {title}");
    }

    fn hz(
        props: &DisplayPageProps,
    ) -> &crate::features::display::components::hz_group::HzGroupProps {
        for block in &props.dashboard.right.blocks {
            if let DashboardBlock::Hz(hz) = block {
                return hz;
            }
        }
        panic!("hz group missing");
    }

    fn feature<'a>(
        props: &'a DisplayPageProps,
        title: &str,
    ) -> &'a crate::features::display::components::feature_grid::FeatureTileProps {
        for block in &props.dashboard.right.blocks {
            if let DashboardBlock::Features(grid) = block {
                return grid
                    .tiles
                    .iter()
                    .find(|t| t.title == title)
                    .unwrap_or_else(|| panic!("feature {title}"));
            }
        }
        panic!("feature grid missing");
    }

    fn segmented_of(row: &SettingRowProps) -> &SegmentedProps {
        match &row.control {
            RowControl::Segmented(s) => s,
            _ => panic!("{} is not segmented", row.title),
        }
    }

    #[test]
    fn selection_prefers_current_then_focused_then_first() {
        let v = view();
        assert_eq!(resolve_selection(&v, Some("DP-3")).as_deref(), Some("DP-3"));
        assert_eq!(resolve_selection(&v, Some("gone")).as_deref(), Some("DP-6"));
        assert_eq!(resolve_selection(&v, None).as_deref(), Some("DP-6"));
        let mut unfocused = v;
        unfocused.outputs[1].focused = false;
        assert_eq!(resolve_selection(&unfocused, None).as_deref(), Some("DP-3"));
        assert_eq!(
            resolve_selection(
                &DisplayView {
                    outputs: vec![],
                    ..unfocused
                },
                None
            ),
            None
        );
    }

    #[test]
    fn active_choices_are_ignored_and_others_dispatch() {
        let log = Rc::new(Log::default());
        let props = page_props(&view(), "DP-6", Hardware::default(), &actions(&log)).unwrap();

        let refresh = hz(&props);
        (refresh.on_pick)(144);
        (segmented_of(mode_row(&props, "Scale")).on_select)(1);
        (segmented_of(mode_row(&props, "Rotation")).on_select)(1);
        let RowControl::Select { on_select, .. } = &mode_row(&props, "Resolution").control else {
            panic!("resolution is a select");
        };
        on_select(0);
        on_select(1);
        let RowControl::Switch { on_toggle, .. } = &mode_row(&props, "Use this display").control
        else {
            panic!("enable is a switch");
        };
        on_toggle(true);
        on_toggle(false);
        (props.profiles.on_open)(1);
        (props.arrangement.on_select)("DP-3".into());
        (props.arrangement.on_drop)("DP-3".into(), 5, -6);

        assert_eq!(
            *log.0.borrow(),
            [
                "hz 144",
                "scale 150",
                "rot 1",
                "res 1280x720",
                "enable false",
                "open default",
                "select DP-3",
                "move DP-3 5,-6"
            ]
        );
    }

    #[test]
    fn flipping_keeps_the_rotation() {
        let log = Rc::new(Log::default());
        let mut v = view();
        v.outputs[1].transform_options[0].active = false;
        v.outputs[1].transform_options[1].active = true;
        let props = page_props(&v, "DP-6", Hardware::default(), &actions(&log)).unwrap();
        let RowControl::Switch { on_toggle, active } = &mode_row(&props, "Flipped").control else {
            panic!("flip is a switch");
        };
        assert!(!active);
        on_toggle(false);
        on_toggle(true);
        // 90° flipped is Hyprland transform 5.
        assert_eq!(*log.0.borrow(), ["rot 5"]);
    }

    #[test]
    fn header_and_tiles_describe_the_outputs() {
        let log = Rc::new(Log::default());
        let mut v = view();
        v.outputs[0].enabled = false;
        v.outputs[0].logical_width = 0;
        v.outputs[0].logical_height = 0;
        let props = page_props(&v, "DP-6", Hardware::default(), &actions(&log)).unwrap();

        assert_eq!(props.output_header.badge.as_deref(), Some("DP-6"));
        assert_eq!(
            props.output_header.subtitle.as_deref(),
            Some("Monitor • Focused • 1920×1080 @ 60 Hz • 0, 0")
        );
        let off = &props.arrangement.tiles[0];
        assert!(!off.enabled);
        assert_eq!(
            (off.x, off.y),
            (1920, 0),
            "a disabled tile sits at its suggested spot"
        );
        assert_eq!(
            (off.width, off.height),
            (1920, 1080),
            "sized from its first resolution"
        );
        assert!(props.arrangement.tiles[1].selected);
        assert!(props.arrangement.tiles[1].focused);

        assert_eq!(props.profiles.rows[0].subtitle, "2 configured outputs");
        assert_eq!(props.profiles.rows[1].subtitle, "1 configured output");
        assert_eq!(props.profiles.rows[0].icon, Icon::House);

        let off_props = page_props(&v, "DP-3", Hardware::default(), &actions(&log)).unwrap();
        assert_eq!(
            off_props.output_header.subtitle.as_deref(),
            Some("Monitor • Disabled")
        );
    }

    #[test]
    fn dropping_resolves_through_the_application_rules() {
        let log = Rc::new(Log::default());
        let mut v = view();
        v.outputs[1].x = 1920;
        let props = page_props(&v, "DP-6", Hardware::default(), &actions(&log)).unwrap();
        // Close to flush against the neighbour: it lands flush and aligned.
        let previewed = (props.arrangement.preview)("DP-6", (1950, 12), 30).unwrap();
        assert!(previewed.contains(&("DP-6".to_owned(), 1920, 0)));
        assert!(previewed.contains(&("DP-3".to_owned(), 0, 0)));
        assert_eq!((props.arrangement.preview)("nope", (0, 0), 30), None);
    }

    #[test]
    fn warnings_become_the_page_notice() {
        let log = Rc::new(Log::default());
        let mut v = view();
        v.warnings = vec!["displays are not connected".into()];
        let props = page_props(&v, "DP-6", Hardware::default(), &actions(&log)).unwrap();
        assert_eq!(props.notice.as_deref(), Some("displays are not connected"));
    }

    #[test]
    fn profile_detail_lists_saved_outputs_and_dispatches() {
        let log = Rc::new(Log::default());
        let detail = profile_detail_props(&view(), "home", &actions(&log)).unwrap();
        assert_eq!(detail.subtitle, "2 displays configured in this arrangement");
        assert_eq!(detail.outputs[0].badge.as_deref(), Some("+1080, 0"));
        assert_eq!(detail.outputs[0].subtitle, "1920x1080@60 · 1×");
        assert!(detail.removable);
        (detail.on_back)();
        (detail.on_apply)();
        (detail.on_delete)();
        assert_eq!(*log.0.borrow(), ["close", "apply home", "delete home"]);

        let default = profile_detail_props(&view(), "default", &actions(&log)).unwrap();
        assert!(!default.removable);
        assert!(profile_detail_props(&view(), "gone", &actions(&log)).is_none());
    }

    fn capability(
        supported: bool,
        enabled: bool,
        reason: &str,
    ) -> settings_domain::controls::Capability {
        settings_domain::controls::Capability {
            supported,
            enabled,
            reason: reason.into(),
        }
    }

    fn level(percent: Option<u8>, reason: &str) -> settings_domain::controls::Control {
        settings_domain::controls::Control {
            kind: settings_domain::controls::ControlKind::Ddc,
            percent,
            locked: false,
            reason: reason.into(),
            code: None,
        }
    }

    fn controls() -> OutputControls {
        OutputControls {
            connector: "DP-6".into(),
            brightness: level(Some(40), ""),
            contrast: level(Some(70), ""),
            overdrive: level(None, "No overdrive"),
            input: settings_domain::controls::InputSources::default(),
            caps: settings_domain::controls::FeatureCaps {
                hdr: capability(true, false, ""),
                ten_bit: capability(false, false, "Display is 8-bit"),
                vrr: capability(true, true, ""),
            },
            port: None,
        }
    }

    #[test]
    fn rows_wait_for_the_probe_then_follow_the_capabilities() {
        let log = Rc::new(Log::default());
        let waiting = page_props(&view(), "DP-6", Hardware::default(), &actions(&log)).unwrap();
        let hdr = feature(&waiting, "HDR");
        assert!(!hdr.sensitive);
        assert_eq!(hdr.tooltip.as_deref(), Some("Checking the display…"));

        let found = controls();
        let hardware = Hardware {
            controls: Some(&found),
            ..Hardware::default()
        };
        let props = page_props(&view(), "DP-6", hardware, &actions(&log)).unwrap();
        let hdr = feature(&props, "HDR");
        assert!(hdr.sensitive);
        assert!(!hdr.active);
        (hdr.on_toggle)(false);
        (hdr.on_toggle)(true);
        assert_eq!(*log.0.borrow(), ["flag Hdr true"]);

        let ten_bit = feature(&props, "10-bit");
        assert!(!ten_bit.sensitive);
        assert_eq!(ten_bit.tooltip.as_deref(), Some("Display is 8-bit"));
        assert!(right_row(&props, "Contrast").sensitive);
        assert!(!right_row(&props, "Overdrive").sensitive);
    }

    #[test]
    fn another_outputs_probe_is_not_shown() {
        let log = Rc::new(Log::default());
        let found = controls();
        let hardware = Hardware {
            controls: Some(&found),
            ..Hardware::default()
        };
        let props = page_props(&view(), "DP-3", hardware, &actions(&log)).unwrap();
        assert!(!feature(&props, "HDR").sensitive);
    }

    #[test]
    fn the_main_display_button_is_taken_by_the_host() {
        let log = Rc::new(Log::default());
        let mut v = view();
        let props = page_props(&v, "DP-6", Hardware::default(), &actions(&log)).unwrap();
        match &right_row(&props, "Main display").control {
            RowControl::Button { on_click, .. } => on_click(),
            _ => panic!("not a button"),
        }
        assert_eq!(*log.0.borrow(), ["main"]);

        v.bar_host = Some("DP-6".into());
        let props = page_props(&v, "DP-6", Hardware::default(), &actions(&log)).unwrap();
        assert!(!right_row(&props, "Main display").sensitive);
    }

    #[test]
    fn global_rows_dispatch_and_the_temperature_needs_the_night_light() {
        let log = Rc::new(Log::default());
        let brightness = BrightnessView {
            percent: Some(30),
            sync: true,
            targets: 2,
        };
        let night = NightLightView {
            on: false,
            kelvin: 4000,
        };
        let hardware = Hardware {
            brightness: Some(&brightness),
            night: Some(&night),
            ..Hardware::default()
        };
        let group = hardware_props::global_env_group(hardware, &actions(&log));
        let by = |title: &str| group.rows.iter().find(|r| r.title == title).unwrap();
        match &by("Brightness").control {
            RowControl::Slider(slider) => (slider.on_change)(55.0),
            _ => panic!("not a slider"),
        }
        match &by("Night light").control {
            RowControl::Switch { on_toggle, .. } => on_toggle(true),
            _ => panic!("not a switch"),
        }
        assert!(group.rows.iter().all(|r| r.title != "Color temperature"));
        assert_eq!(*log.0.borrow(), ["brightness 55", "night true"]);

        let lit = NightLightView {
            on: true,
            kelvin: 3500,
        };
        let hardware = Hardware {
            night: Some(&lit),
            ..Hardware::default()
        };
        let group = hardware_props::global_env_group(hardware, &actions(&log));
        assert!(
            group
                .rows
                .iter()
                .find(|r| r.title == "Color temperature")
                .unwrap()
                .sensitive
        );
    }

    #[test]
    fn dashboard_places_global_in_the_left_column_without_refresh_rows() {
        let log = Rc::new(Log::default());
        let props = page_props(&view(), "DP-6", Hardware::default(), &actions(&log)).unwrap();
        assert_eq!(props.dashboard.left.blocks.len(), 2);
        let DashboardBlock::Group(global) = &props.dashboard.left.blocks[1] else {
            panic!("global env group");
        };
        assert_eq!(global.rows[0].title, "Global Environment");
        let DashboardBlock::Group(mode) = &props.dashboard.left.blocks[0] else {
            panic!("mode group");
        };
        assert!(
            mode.rows
                .iter()
                .all(|r| r.title != "Refresh Rate" && r.title != "Custom refresh rates")
        );
        hz(&props);
        feature(&props, "HDR");
    }

    #[test]
    fn hz_group_props_dispatch_pick_add_and_remove() {
        let log = Rc::new(RefCell::new(Vec::<String>::new()));
        let pick_log = Rc::clone(&log);
        let add_log = Rc::clone(&log);
        let remove_log = Rc::clone(&log);
        let props = crate::features::display::components::hz_group::HzGroupProps {
            enabled: true,
            current_hz: 60,
            options: vec![
                crate::features::display::components::hz_group::HzRateOption {
                    hz: 60,
                    active: true,
                    custom: false,
                },
            ],
            on_pick: Rc::new(move |hz| pick_log.borrow_mut().push(format!("pick {hz}"))),
            on_add_custom: Rc::new(move |hz| add_log.borrow_mut().push(format!("add {hz}"))),
            on_remove_custom: Rc::new(move |hz| {
                remove_log.borrow_mut().push(format!("remove {hz}"))
            }),
        };
        (props.on_pick)(144);
        (props.on_add_custom)(165);
        (props.on_remove_custom)(144);
        assert_eq!(*log.borrow(), vec!["pick 144", "add 165", "remove 144"]);
    }

    #[test]
    fn feature_tiles_expose_support_state() {
        let toggled = Rc::new(RefCell::new(false));
        let on_toggle = {
            let toggled = Rc::clone(&toggled);
            Rc::new(move |value: bool| *toggled.borrow_mut() = value)
        };
        let tile = crate::features::display::components::feature_grid::FeatureTileProps {
            title: "HDR",
            subtitle: "High dynamic range",
            icon: Icon::Sun,
            active: false,
            sensitive: false,
            tooltip: Some("Unsupported".into()),
            on_toggle,
        };
        assert!(!tile.sensitive);
        (tile.on_toggle)(true);
        assert!(*toggled.borrow());
    }
}
