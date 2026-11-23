use settings_domain::network::{NetworkPreferences, ScanPolicy};

use super::NetworkCore;
use crate::error::AppError;
use crate::network_dto::DiagnosticsView;

const JOURNAL_LINES: usize = 80;

impl NetworkCore {
    pub(crate) fn network_diagnostics(&self) -> Result<DiagnosticsView, AppError> {
        let journal = match self.ports.journal.lines(JOURNAL_LINES) {
            Ok(lines) => lines,
            Err(err) => {
                self.error(format!("Could not read the NetworkManager journal: {err}"));
                Vec::new()
            }
        };
        Ok(DiagnosticsView {
            actions: self.session().log.entries(),
            journal,
        })
    }

    pub(crate) fn subscribe(
        &self,
        sink: Box<dyn Fn() + Send>,
    ) -> Result<settings_domain::ports::Subscription, AppError> {
        Ok(self.ports.events.subscribe(sink)?)
    }

    pub(crate) fn clear_network_log(&self) {
        self.session().log.clear();
    }

    pub(crate) fn scan_preferences(&self) -> Result<NetworkPreferences, AppError> {
        Ok(self.ports.prefs.load()?.normalized())
    }

    pub(crate) fn set_scan_policy(
        &self,
        policy: ScanPolicy,
        interval_ms: u32,
    ) -> Result<(), AppError> {
        let mut prefs = self.ports.prefs.load()?;
        prefs.policy = policy;
        prefs.interval_ms = interval_ms;
        self.ports.prefs.save(&prefs.normalized())?;
        Ok(())
    }
}
