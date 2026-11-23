//! Network use cases. Rules live in `settings_domain::network`; this layer sequences ports
//! and keeps the session state: the selected adapters, the scan guard and the action log.

mod diagnostics;
mod ethernet;
mod overview;
mod profile;
mod vpn;
mod wifi;

use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use settings_domain::network::{AccessPoint, ActionLog, LogLevel};

use crate::network_ports::NetworkPorts;

/// Counter reading the rates are measured against.
pub(super) struct RateSample {
    pub at: Instant,
    pub iface: String,
    pub rx: u64,
    pub tx: u64,
}

#[derive(Default)]
pub(super) struct Session {
    pub wifi_iface: String,
    pub eth_iface: String,
    pub scanning: bool,
    pub last_scan: Option<Instant>,
    /// The last non-empty list, shown again when a read comes back empty mid-scan.
    pub cached_aps: Vec<AccessPoint>,
    pub primary_iface: String,
    pub sample: Option<RateSample>,
    pub log: ActionLog,
}

pub(crate) struct NetworkCore {
    pub(super) ports: NetworkPorts,
    session: Mutex<Session>,
    /// Edits that delete and re-import a tunnel must not interleave.
    pub(super) vpn_lock: Mutex<()>,
}

impl NetworkCore {
    pub(crate) fn new(ports: NetworkPorts) -> Self {
        Self {
            ports,
            session: Mutex::new(Session::default()),
            vpn_lock: Mutex::new(()),
        }
    }

    pub(super) fn session(&self) -> MutexGuard<'_, Session> {
        self.session.lock().expect("network session")
    }

    pub(super) fn log(&self, level: LogLevel, text: impl Into<String>) {
        let now = self.ports.clock.unix_secs();
        self.session().log.push(level, text, now);
    }

    pub(super) fn info(&self, text: impl Into<String>) {
        self.log(LogLevel::Info, text);
    }

    pub(super) fn warn(&self, text: impl Into<String>) {
        self.log(LogLevel::Warn, text);
    }

    pub(super) fn error(&self, text: impl Into<String>) {
        self.log(LogLevel::Error, text);
    }
}
