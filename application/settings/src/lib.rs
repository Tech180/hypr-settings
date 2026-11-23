//! Use cases for display settings. Orchestration only: rules live in `settings-domain`,
//! IO lives behind its driven ports.

pub mod api;
mod bluetooth_ports;
mod bluetooth_service;
pub mod drag;
pub mod dto;
pub mod error;
pub mod network_dto;
mod network_ports;
mod network_service;
pub mod ports;
pub mod service;
mod settings_service;
mod use_cases;

pub use api::{BluetoothApi, ControlsApi, DisplayApi, NetworkApi, SettingsApi};
pub use bluetooth_ports::BluetoothPorts;
pub use bluetooth_service::BluetoothService;
pub use error::AppError;
pub use network_ports::NetworkPorts;
pub use network_service::NetworkService;
pub use ports::Ports;
pub use service::DisplayService;
pub use settings_service::SettingsService;
