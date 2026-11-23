use std::io::Write;

use settings_application::SettingsApi;

use crate::args::BluetoothCommand;
use crate::display::CliError;

pub fn run(
    api: &dyn SettingsApi,
    command: BluetoothCommand,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    match command {
        BluetoothCommand::List { json } => {
            let view = api.bluetooth_view()?;
            if json {
                writeln!(out, "{}", serde_json::to_string_pretty(&view)?)?;
            } else {
                if let Some(adapter) = &view.adapter {
                    writeln!(out, "adapter: {} ({})", adapter.name, adapter.address)?;
                }
                writeln!(out, "powered: {}", view.powered)?;
                writeln!(out, "alias: {}", view.alias)?;
                for device in view.paired.iter().chain(view.discovered.iter()) {
                    writeln!(
                        out,
                        "{}: {} ({})",
                        device.id,
                        device.name,
                        if device.connected {
                            "connected"
                        } else if device.paired {
                            "paired"
                        } else {
                            "discovered"
                        }
                    )?;
                }
            }
            Ok(())
        }
        BluetoothCommand::Power { on, off } => {
            let enabled = on || !off;
            api.set_bluetooth_powered(enabled)?;
            writeln!(
                out,
                "bluetooth powered {}",
                if enabled { "on" } else { "off" }
            )?;
            Ok(())
        }
        BluetoothCommand::Connect { address } => {
            api.connect_device(settings_domain::DeviceId::new(address))?;
            writeln!(out, "connect requested")?;
            Ok(())
        }
        BluetoothCommand::Adapters { json } => {
            let view = api.bluetooth_view()?;
            if json {
                writeln!(out, "{}", serde_json::to_string_pretty(&view.adapters)?)?;
            } else {
                let active = view.adapter.as_ref().map(|a| a.address.as_str());
                for adapter in &view.adapters {
                    let mark = active == Some(adapter.address.as_str());
                    writeln!(
                        out,
                        "{}{}: {} ({}) powered={}",
                        if mark { "* " } else { "  " },
                        adapter.address,
                        adapter.name,
                        adapter.alias,
                        adapter.powered
                    )?;
                }
            }
            Ok(())
        }
        BluetoothCommand::Adapter { address } => {
            api.select_bluetooth_adapter(address.clone())?;
            writeln!(out, "active adapter set to {address}")?;
            Ok(())
        }
    }
}
