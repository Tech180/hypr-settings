use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "hypr-settings",
    version,
    about = "Native Hyprland settings window"
)]
pub struct Cli {
    /// Without a subcommand the settings window opens.
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Inspect and change displays without opening the window.
    Display {
        #[command(subcommand)]
        command: DisplayCommand,
    },
    /// Inspect and change Bluetooth without opening the window.
    Bluetooth {
        #[command(subcommand)]
        command: BluetoothCommand,
    },
    /// Inspect and change networking (Wi-Fi, VPN) without opening the window.
    Network {
        #[command(subcommand)]
        command: NetworkCommand,
    },
    /// Open the settings window (or focus it when already open).
    Open {
        /// Section to show, for example `display`.
        #[arg(long)]
        section: Option<String>,
    },
    /// Open the settings window, or close it when it is already open.
    Toggle,
    /// Close the settings window if it is open.
    Close,
    /// Reload live data in the open settings window.
    Refresh,
}

#[derive(Debug, Subcommand)]
pub enum DisplayCommand {
    /// List outputs and the active kanshi profile.
    List {
        #[arg(long)]
        json: bool,
    },
    /// List kanshi profiles.
    Profiles {
        #[arg(long)]
        json: bool,
    },
    /// Change one output.
    Set(SetArgs),
    /// Move one output to a position in the layout (logical pixels).
    Move(MoveArgs),
    /// Undo a trial that a crashed session left behind (run at login).
    RevertPending,
    /// Make an output the main display: the one that hosts the bar.
    Main {
        /// Connector name, for example DP-3.
        connector: String,
    },
    /// Store the live layout in the active profile (or `home` when none matches).
    Save,
    /// Extra refresh rates that are offered for an output.
    Hz {
        #[command(subcommand)]
        command: HzCommand,
    },
    /// Turn HDR, 10-bit color or Adaptive Sync on or off for an output.
    Flag(FlagArgs),
    /// Put back the stored HDR / 10-bit / Adaptive Sync settings and a manual night light
    /// (run at login and after a profile change).
    Restore,
    /// Brightness of the displays.
    Brightness {
        #[command(subcommand)]
        command: BrightnessCommand,
    },
    /// Contrast of an output over DDC/CI.
    Contrast(PercentArgs),
    /// Overdrive of an output over DDC/CI.
    Overdrive(PercentArgs),
    /// Input source of an output over DDC/CI.
    Input(InputArgs),
    /// Night light.
    Night {
        #[command(subcommand)]
        command: NightCommand,
    },
    /// Show the hardware controls and the port of one output.
    Controls {
        connector: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum HzCommand {
    /// Offer a refresh rate that the output advertises.
    Add {
        connector: String,
        hz: u32,
    },
    Remove {
        connector: String,
        hz: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FlagName {
    Hdr,
    #[value(name = "10bit")]
    TenBit,
    Vrr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Toggle {
    On,
    Off,
}

#[derive(Debug, Args)]
pub struct FlagArgs {
    pub connector: String,
    pub flag: FlagName,
    pub state: Toggle,
}

#[derive(Debug, Subcommand)]
pub enum BrightnessCommand {
    /// Print the current brightness.
    Get {
        #[arg(long)]
        json: bool,
    },
    /// Set the brightness in percent.
    Set { percent: u8 },
    /// Change the brightness by a number of percent points, for example `+5` or `-5`.
    Adjust {
        #[arg(allow_hyphen_values = true)]
        delta: i32,
    },
    /// Look for displays again and update what the brightness keys use (run after a plug).
    Refresh,
}

#[derive(Debug, Args)]
pub struct PercentArgs {
    pub connector: String,
    pub percent: u8,
}

#[derive(Debug, Args)]
pub struct InputArgs {
    pub connector: String,
    /// The VCP value, for example `0x0f` or `17`.
    pub source: String,
}

#[derive(Debug, Subcommand)]
pub enum NightCommand {
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Hold a fixed temperature instead of the schedule.
    On,
    /// Go back to the schedule.
    Off,
    /// Set the temperature in kelvin (3000 to 6500).
    Set { kelvin: u32 },
}

#[derive(Debug, Args)]
#[command(group(clap::ArgGroup::new("toggle").args(["enable", "disable"])))]
pub struct SetArgs {
    /// Connector name, for example eDP-1.
    #[arg(long)]
    pub connector: String,
    /// `WxH@R` for an exact mode, or `WxH` to keep the current refresh rate.
    #[arg(long)]
    pub mode: Option<String>,
    /// Refresh rate in whole hertz at the current resolution.
    #[arg(long, conflicts_with = "mode")]
    pub refresh: Option<u32>,
    #[arg(long)]
    pub scale: Option<f64>,
    /// Rotation in degrees: 0, 90, 180 or 270.
    #[arg(long)]
    pub rotate: Option<u32>,
    /// Mirror the output (combine with --rotate; without it the rotation is 0).
    #[arg(long)]
    pub flip: bool,
    /// Layout position, in logical pixels. Needs --y.
    #[arg(long, requires = "y", allow_hyphen_values = true)]
    pub x: Option<i32>,
    /// Layout position, in logical pixels. Needs --x.
    #[arg(long, requires = "x", allow_hyphen_values = true)]
    pub y: Option<i32>,
    #[arg(long)]
    pub enable: bool,
    #[arg(long)]
    pub disable: bool,
    /// Revert automatically unless confirmed with Enter.
    #[arg(long)]
    pub trial: bool,
}

#[derive(Debug, Args)]
pub struct MoveArgs {
    /// Connector name, for example DP-3.
    pub connector: String,
    #[arg(allow_hyphen_values = true)]
    pub x: i32,
    #[arg(allow_hyphen_values = true)]
    pub y: i32,
    /// Revert automatically unless confirmed with Enter.
    #[arg(long)]
    pub trial: bool,
}

#[derive(Debug, Subcommand)]
pub enum BluetoothCommand {
    /// List adapter state and devices.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Turn the default adapter on or off.
    Power {
        #[arg(long, conflicts_with = "off")]
        on: bool,
        #[arg(long, conflicts_with = "on")]
        off: bool,
    },
    /// Connect to a paired device by MAC address.
    Connect {
        /// Device address, for example `AA:BB:CC:DD:EE:FF`.
        address: String,
    },
    /// List Bluetooth adapters on this system.
    Adapters {
        #[arg(long)]
        json: bool,
    },
    /// Select the active adapter by MAC address (saved in preferences).
    Adapter {
        /// Adapter address, for example `AA:BB:CC:DD:EE:FF`.
        address: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum NetworkCommand {
    /// Show the active link, connectivity and tunnel count.
    Status,
    /// Wi-Fi radio, scanning and connections.
    Wifi {
        #[command(subcommand)]
        command: WifiCommand,
    },
    /// WireGuard and OpenVPN tunnels.
    Vpn {
        #[command(subcommand)]
        command: VpnCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum WifiCommand {
    /// List visible networks.
    List,
    /// Turn the Wi-Fi radio on.
    On,
    /// Turn the Wi-Fi radio off.
    Off,
    /// Ask the radio to scan now.
    Scan,
    /// Join a network; a saved network needs no password.
    Connect {
        ssid: String,
        #[arg(long)]
        password: Option<String>,
    },
    /// Leave the current network.
    Disconnect,
}

#[derive(Debug, Subcommand)]
pub enum VpnCommand {
    /// List tunnels and whether they are up.
    List,
    /// Bring a tunnel up.
    Up { id: String },
    /// Bring a tunnel down.
    Down { id: String },
}
