//! Driven adapter for NetworkManager (`nmcli`), `wg-quick` units (`systemctl`), the
//! NetworkManager journal, sysfs counters, QR encoding and nearby VPN configs.

mod connections;
mod devices;
pub mod events;
pub mod exec;
pub mod gateway;
mod nm;
mod parse;
pub mod system;
pub mod systemd;
pub mod vpn;
mod wifi;

pub use events::NmEvents;
pub use exec::{Exec, ExecError, SystemExec};
pub use gateway::NmGateway;
pub use nm::Nm;
pub use system::{FsVpnFiles, JournalReader, QrCodeEncoder, SysfsStats};
pub use systemd::Systemctl;
pub use vpn::NmVpn;
