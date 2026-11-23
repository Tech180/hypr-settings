//! Headless driving adapter: the command line definition and the display commands.
//! Window commands (`open`, `toggle`, ...) are dispatched by the binary to the GTK adapter.

pub mod args;
mod bluetooth;
mod controls;
pub mod display;
mod network;

pub use args::{
    BluetoothCommand, BrightnessCommand, Cli, Command, DisplayCommand, FlagArgs, FlagName,
    HzCommand, InputArgs, MoveArgs, NetworkCommand, NightCommand, PercentArgs, SetArgs, Toggle,
    VpnCommand, WifiCommand,
};
pub use bluetooth::run as run_bluetooth;
pub use display::{CliError, build_patch, run as run_display};
pub use network::run as run_network;

#[cfg(test)]
mod tests;
