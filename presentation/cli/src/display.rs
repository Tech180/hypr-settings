use std::fmt;
use std::io::{self, BufRead, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::json;
use settings_application::dto::{
    ApplyOutcome, DisplayView, OutputView, RecoverOutcome, TickOutcome,
};
use settings_application::{AppError, DisplayApi, SettingsApi};
use settings_domain::{
    ConnectorName, LayoutPatch, Mode, ModeRequest, Position, Resolution, Scale, Transform,
};

use crate::args::{DisplayCommand, MoveArgs, SetArgs};

#[derive(Debug)]
pub struct CliError(pub String);

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CliError {}

impl From<AppError> for CliError {
    fn from(err: AppError) -> Self {
        Self(err.to_string())
    }
}

impl From<io::Error> for CliError {
    fn from(err: io::Error) -> Self {
        Self(err.to_string())
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        Self(err.to_string())
    }
}

pub fn run(
    api: &dyn SettingsApi,
    command: DisplayCommand,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    match command {
        DisplayCommand::List { json } => list(&api.view()?, json, out),
        DisplayCommand::Profiles { json } => profiles(&api.view()?, json, out),
        DisplayCommand::Set(args) => set(api, &args, out),
        DisplayCommand::Move(args) => move_output(api, &args, out),
        DisplayCommand::RevertPending => revert_pending(api, out),
        other => crate::controls::run(api, other, out),
    }
}

fn mode_label(o: &OutputView) -> String {
    if o.width == 0 {
        "off".to_owned()
    } else {
        format!("{}x{}@{}", o.width, o.height, o.refresh_hz)
    }
}

fn active_chip<T>(chips: &[T], is_active: impl Fn(&T) -> bool) -> Option<&T> {
    chips.iter().find(|c| is_active(c))
}

fn scale_label(o: &OutputView) -> String {
    active_chip(&o.scale_options, |c| c.active).map_or_else(|| "?".to_owned(), |c| c.label.clone())
}

fn transform_label(o: &OutputView) -> String {
    let rotation = active_chip(&o.transform_options, |c| c.active).map_or("?", |c| c.label);
    if o.flipped {
        format!("{rotation} mirrored")
    } else {
        rotation.to_owned()
    }
}

fn list(view: &DisplayView, as_json: bool, out: &mut dyn Write) -> Result<(), CliError> {
    if as_json {
        let outputs: Vec<_> = view
            .outputs
            .iter()
            .map(|o| {
                json!({
                    "connector": o.connector,
                    "description": o.description,
                    "enabled": o.enabled,
                    "focused": o.focused,
                    "width": o.width,
                    "height": o.height,
                    "refreshHz": o.refresh_hz,
                    "x": o.x,
                    "y": o.y,
                    "logicalWidth": o.logical_width,
                    "logicalHeight": o.logical_height,
                    "flipped": o.flipped,
                    "scale": active_chip(&o.scale_options, |c| c.active)
                        .map(|c| f64::from(c.hundredths) / 100.0),
                    "transform": active_chip(&o.transform_options, |c| c.active).map(|c| c.hypr),
                    "resolutions": o.resolutions.iter().map(|r| format!("{}x{}", r.width, r.height)).collect::<Vec<_>>(),
                    "hzOptions": o.hz_options.iter().map(|c| c.hz).collect::<Vec<_>>(),
                    "customHz": o.hz_options.iter().filter(|c| c.custom).map(|c| c.hz).collect::<Vec<_>>(),
                })
            })
            .collect();
        let doc = json!({
            "activeProfile": view.active_profile,
            "barHost": view.bar_host,
            "warnings": view.warnings,
            "outputs": outputs,
        });
        writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&doc).map_err(|e| CliError(e.to_string()))?
        )?;
        return Ok(());
    }

    for o in &view.outputs {
        writeln!(
            out,
            "{:<10} {:<4} {:<16} at {},{}  scale {}  rotate {}{}  {}",
            o.connector,
            if o.enabled { "on" } else { "off" },
            mode_label(o),
            o.x,
            o.y,
            scale_label(o),
            transform_label(o),
            if o.focused { "  (focused)" } else { "" },
            o.description,
        )?;
    }
    writeln!(
        out,
        "profile: {}",
        view.active_profile.as_deref().unwrap_or("(none matched)")
    )?;
    if let Some(host) = &view.bar_host {
        writeln!(out, "main display: {host}")?;
    }
    for warning in &view.warnings {
        writeln!(out, "warning: {warning}")?;
    }
    Ok(())
}

fn profiles(view: &DisplayView, as_json: bool, out: &mut dyn Write) -> Result<(), CliError> {
    if as_json {
        let items: Vec<_> = view
            .profiles
            .iter()
            .map(|p| json!({ "name": p.name, "outputs": p.output_count, "active": p.active }))
            .collect();
        writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&items).map_err(|e| CliError(e.to_string()))?
        )?;
        return Ok(());
    }
    for p in &view.profiles {
        writeln!(
            out,
            "{} {:<12} {} output(s)",
            if p.active { "*" } else { " " },
            p.name,
            p.output_count
        )?;
    }
    Ok(())
}

/// Turns CLI flags into a layout patch. Public so the flag handling can be tested.
pub fn build_patch(args: &SetArgs) -> Result<LayoutPatch, CliError> {
    let connector =
        ConnectorName::try_from(args.connector.as_str()).map_err(|e| CliError(e.to_string()))?;
    let mut patch = LayoutPatch::new(connector);

    if let Some(text) = &args.mode {
        let request =
            if text.contains('@') {
                ModeRequest::Exact(text.parse::<Mode>().map_err(|e| CliError(e.to_string()))?)
            } else {
                ModeRequest::Resolution(Resolution::parse(text).ok_or_else(|| {
                    CliError(format!("invalid mode {text:?}; expected WxH or WxH@R"))
                })?)
            };
        patch = patch.mode(request);
    }
    if let Some(hz) = args.refresh {
        patch = patch.mode(ModeRequest::Refresh(hz));
    }
    if let Some(scale) = args.scale {
        patch = patch.scale(Scale::new(scale).map_err(|e| CliError(e.to_string()))?);
    }
    if args.rotate.is_some() || args.flip {
        let rotation = match args.rotate.unwrap_or(0) {
            0 => Transform::Normal,
            90 => Transform::R90,
            180 => Transform::R180,
            270 => Transform::R270,
            other => {
                return Err(CliError(format!(
                    "rotation must be 0, 90, 180 or 270, got {other}"
                )));
            }
        };
        patch = patch.transform(rotation.with_flipped(args.flip));
    }
    if let (Some(x), Some(y)) = (args.x, args.y) {
        patch = patch.position(Position::new(x, y));
    }
    if args.enable {
        patch = patch.enable(true);
    }
    if args.disable {
        patch = patch.enable(false);
    }
    Ok(patch)
}

fn outcome_note(outcome: ApplyOutcome) -> &'static str {
    match outcome {
        ApplyOutcome::Verified => "applied",
        ApplyOutcome::Unverified => "applied, but Hyprland has not confirmed it yet",
        ApplyOutcome::Degraded => "applied directly (kanshi was unavailable)",
    }
}

fn set(api: &dyn DisplayApi, args: &SetArgs, out: &mut dyn Write) -> Result<(), CliError> {
    let patch = build_patch(args)?;
    let only_toggle = patch.mode.is_none()
        && patch.scale.is_none()
        && patch.transform.is_none()
        && patch.position.is_none();

    if only_toggle {
        let Some(enable) = patch.enable else {
            return Err(CliError(
                "nothing to change; pass --mode, --refresh, --scale, --rotate, --flip, --x/--y, --enable or --disable".into(),
            ));
        };
        let outcome = api.set_enabled(patch.connector.clone(), enable)?;
        writeln!(out, "{}: {}", patch.connector, outcome_note(outcome))?;
        return Ok(());
    }

    run_trial(api, patch, args.trial, out)
}

fn move_output(api: &dyn DisplayApi, args: &MoveArgs, out: &mut dyn Write) -> Result<(), CliError> {
    let connector =
        ConnectorName::try_from(args.connector.as_str()).map_err(|e| CliError(e.to_string()))?;
    let patch = LayoutPatch::new(connector).position(Position::new(args.x, args.y));
    run_trial(api, patch, args.trial, out)
}

fn run_trial(
    api: &dyn DisplayApi,
    patch: LayoutPatch,
    confirm: bool,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    let trial = api.begin_trial(patch)?;
    if !confirm {
        api.keep_trial()?;
        writeln!(out, "{}: {}", trial.connector, outcome_note(trial.outcome))?;
        return Ok(());
    }
    writeln!(
        out,
        "{}: {}. Press Enter within {}s to keep it; otherwise it is reverted.",
        trial.connector,
        outcome_note(trial.outcome),
        trial.timeout_secs
    )?;
    wait_for_confirmation(api, out)
}

fn wait_for_confirmation(api: &dyn DisplayApi, out: &mut dyn Write) -> Result<(), CliError> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut line = String::new();
        if io::stdin()
            .lock()
            .read_line(&mut line)
            .is_ok_and(|read| read > 0)
        {
            let _ = tx.send(());
        }
    });

    loop {
        if rx.recv_timeout(Duration::from_secs(1)).is_ok() {
            api.keep_trial()?;
            writeln!(out, "kept")?;
            return Ok(());
        }
        match api.tick()? {
            TickOutcome::Active(view) => {
                write!(out, "\r{}s left ", view.remaining_secs)?;
                out.flush()?;
            }
            TickOutcome::Expired => {
                writeln!(out, "\nreverted")?;
                return Ok(());
            }
            TickOutcome::Idle => return Ok(()),
        }
    }
}

fn revert_pending(api: &dyn DisplayApi, out: &mut dyn Write) -> Result<(), CliError> {
    match api.recover_pending_trial()? {
        RecoverOutcome::Nothing => writeln!(out, "no pending trial")?,
        RecoverOutcome::Restored { connector } => {
            writeln!(
                out,
                "restored the layout from before the trial on {connector}"
            )?;
        }
    }
    Ok(())
}
