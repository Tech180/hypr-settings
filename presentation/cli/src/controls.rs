//! The hardware side of the display commands: main display, flags, brightness, DDC/CI
//! controls and the night light.

use std::io::Write;

use serde_json::json;
use settings_application::SettingsApi;
use settings_application::dto::OutputControls;
use settings_domain::controls::{Control, Flag, port_line, port_tooltip};
use settings_domain::{ConnectorName, Kelvin};

use crate::args::{
    BrightnessCommand, DisplayCommand, FlagArgs, FlagName, HzCommand, InputArgs, NightCommand,
    PercentArgs, Toggle,
};
use crate::display::CliError;

fn connector(name: &str) -> Result<ConnectorName, CliError> {
    ConnectorName::try_from(name).map_err(|e| CliError(e.to_string()))
}

pub(crate) fn run(
    api: &dyn SettingsApi,
    command: DisplayCommand,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    match command {
        DisplayCommand::Main { connector: name } => {
            api.set_bar_host(connector(&name)?)?;
            writeln!(out, "{name} hosts the bar now")?;
        }
        DisplayCommand::Save => {
            let name = api.save_to_active_profile()?;
            writeln!(out, "layout stored in profile {name}")?;
        }
        DisplayCommand::Hz { command } => match command {
            HzCommand::Add {
                connector: name,
                hz,
            } => {
                api.add_custom_hz(connector(&name)?, hz)?;
                writeln!(out, "{name}: {hz} Hz is offered")?;
            }
            HzCommand::Remove {
                connector: name,
                hz,
            } => {
                api.remove_custom_hz(connector(&name)?, hz)?;
                writeln!(out, "{name}: {hz} Hz removed")?;
            }
        },
        DisplayCommand::Flag(args) => flag(api, &args, out)?,
        DisplayCommand::Restore => restore(api, out)?,
        DisplayCommand::Brightness { command } => brightness(api, command, out)?,
        DisplayCommand::Contrast(args) => percent(api, &args, Percent::Contrast, out)?,
        DisplayCommand::Overdrive(args) => percent(api, &args, Percent::Overdrive, out)?,
        DisplayCommand::Input(args) => input(api, &args, out)?,
        DisplayCommand::Night { command } => night(api, command, out)?,
        DisplayCommand::Controls {
            connector: name,
            json,
        } => controls(&api.output_controls(connector(&name)?)?, json, out)?,
        DisplayCommand::List { .. }
        | DisplayCommand::Profiles { .. }
        | DisplayCommand::Set(_)
        | DisplayCommand::Move(_)
        | DisplayCommand::RevertPending => unreachable!("handled by the layout commands"),
    }
    Ok(())
}

fn flag(api: &dyn SettingsApi, args: &FlagArgs, out: &mut dyn Write) -> Result<(), CliError> {
    let which = match args.flag {
        FlagName::Hdr => Flag::Hdr,
        FlagName::TenBit => Flag::TenBit,
        FlagName::Vrr => Flag::Vrr,
    };
    let on = args.state == Toggle::On;
    api.set_flag(connector(&args.connector)?, which, on)?;
    writeln!(
        out,
        "{}: {} {}",
        args.connector,
        which.label(),
        if on { "on" } else { "off" }
    )?;
    Ok(())
}

fn restore(api: &dyn SettingsApi, out: &mut dyn Write) -> Result<(), CliError> {
    // Both are attempted so one failing does not hide the other.
    let flags = api.restore_flags();
    let night = api.restore_night_light();
    match &flags {
        Ok(0) => writeln!(out, "display flags: nothing to change")?,
        Ok(n) => writeln!(out, "display flags: restored on {n} output(s)")?,
        Err(err) => writeln!(out, "display flags: {err}")?,
    }
    match &night {
        Ok(()) => writeln!(out, "night light: ok")?,
        Err(err) => writeln!(out, "night light: {err}")?,
    }
    flags.map(|_| ())?;
    night?;
    Ok(())
}

fn brightness(
    api: &dyn SettingsApi,
    command: BrightnessCommand,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    match command {
        BrightnessCommand::Get { json } => {
            let view = api.brightness()?;
            if json {
                let doc = json!({
                    "percent": view.percent,
                    "sync": view.sync,
                    "targets": view.targets,
                });
                writeln!(
                    out,
                    "{}",
                    serde_json::to_string_pretty(&doc).map_err(|e| CliError(e.to_string()))?
                )?;
            } else {
                match view.percent {
                    Some(p) => writeln!(
                        out,
                        "{p}%  ({} display(s), sync {})",
                        view.targets,
                        on_off(view.sync)
                    )?,
                    None => writeln!(out, "unknown  ({} display(s))", view.targets)?,
                }
            }
        }
        BrightnessCommand::Set { percent } => {
            api.set_brightness(percent)?;
            writeln!(out, "{}%", percent.min(100))?;
        }
        BrightnessCommand::Adjust { delta } => {
            writeln!(out, "{}%", api.adjust_brightness(delta)?)?;
        }
        BrightnessCommand::Refresh => {
            let view = api.refresh_brightness()?;
            writeln!(
                out,
                "{} display(s) can change their brightness",
                view.targets
            )?;
        }
    }
    Ok(())
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

enum Percent {
    Contrast,
    Overdrive,
}

fn percent(
    api: &dyn SettingsApi,
    args: &PercentArgs,
    kind: Percent,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    let target = connector(&args.connector)?;
    let name = match kind {
        Percent::Contrast => {
            api.set_contrast(target, args.percent)?;
            "contrast"
        }
        Percent::Overdrive => {
            api.set_overdrive(target, args.percent)?;
            "overdrive"
        }
    };
    writeln!(out, "{}: {name} {}%", args.connector, args.percent.min(100))?;
    Ok(())
}

/// `0x0f`, `0F`-style hex with a prefix, or plain decimal.
pub(crate) fn parse_source(text: &str) -> Result<u8, CliError> {
    let text = text.trim();
    let parsed = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u8::from_str_radix(hex, 16),
        None => text.parse(),
    };
    parsed.map_err(|_| CliError(format!("invalid input source {text:?}; use 0x0f or 15")))
}

fn input(api: &dyn SettingsApi, args: &InputArgs, out: &mut dyn Write) -> Result<(), CliError> {
    let code = parse_source(&args.source)?;
    api.set_input_source(connector(&args.connector)?, code)?;
    writeln!(out, "{}: input 0x{code:02x}", args.connector)?;
    Ok(())
}

fn night(
    api: &dyn SettingsApi,
    command: NightCommand,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    match command {
        NightCommand::Status { json } => {
            let view = api.night_light()?;
            if json {
                let doc = json!({ "on": view.on, "kelvin": view.kelvin });
                writeln!(out, "{doc}")?;
            } else {
                writeln!(out, "night light {} at {} K", on_off(view.on), view.kelvin)?;
            }
        }
        NightCommand::On => {
            api.set_night_light(true)?;
            writeln!(out, "night light on")?;
        }
        NightCommand::Off => {
            api.set_night_light(false)?;
            writeln!(out, "night light follows its schedule")?;
        }
        NightCommand::Set { kelvin } => {
            let kelvin = Kelvin::clamped(kelvin).get();
            api.set_night_kelvin(kelvin)?;
            writeln!(out, "{kelvin} K")?;
        }
    }
    Ok(())
}

fn control_line(label: &str, control: &Control) -> String {
    match control.percent {
        Some(p) => format!("{label:<11}{p}%"),
        None => format!("{label:<11}unavailable ({})", control.reason),
    }
}

fn controls(view: &OutputControls, as_json: bool, out: &mut dyn Write) -> Result<(), CliError> {
    if as_json {
        let cap = |c: &settings_domain::controls::Capability| json!({ "supported": c.supported, "enabled": c.enabled, "reason": c.reason });
        let level = |c: &Control| json!({ "percent": c.percent, "reason": c.reason });
        let doc = json!({
            "connector": view.connector,
            "brightness": level(&view.brightness),
            "contrast": level(&view.contrast),
            "overdrive": level(&view.overdrive),
            "input": {
                "current": view.input.current.as_ref().map(|i| &i.label),
                "options": view.input.options.iter().map(|i| json!({"label": i.label, "code": i.code})).collect::<Vec<_>>(),
            },
            "hdr": cap(&view.caps.hdr),
            "tenBit": cap(&view.caps.ten_bit),
            "vrr": cap(&view.caps.vrr),
            "port": view.port.as_ref().map(|p| json!({
                "standard": p.standard,
                "version": p.version,
                "link": p.link,
                "sinkInput": p.sink_input,
            })),
        });
        writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&doc).map_err(|e| CliError(e.to_string()))?
        )?;
        return Ok(());
    }
    writeln!(out, "{}", view.connector)?;
    writeln!(out, "{}", control_line("brightness", &view.brightness))?;
    writeln!(out, "{}", control_line("contrast", &view.contrast))?;
    writeln!(out, "{}", control_line("overdrive", &view.overdrive))?;
    if let Some(current) = &view.input.current {
        writeln!(out, "{:<11}{}", "input", current.label)?;
    }
    for (label, cap) in [
        ("HDR", &view.caps.hdr),
        ("10-bit", &view.caps.ten_bit),
        ("VRR", &view.caps.vrr),
    ] {
        writeln!(
            out,
            "{label:<11}{} ({})",
            if cap.enabled { "on" } else { "off" },
            if cap.supported {
                cap.reason.as_str()
            } else {
                "unsupported"
            }
        )?;
    }
    if let Some(port) = &view.port {
        writeln!(out, "{:<11}{} {}", "port", port.standard, port_line(port))?;
        writeln!(out, "{}", port_tooltip(port))?;
    }
    Ok(())
}
