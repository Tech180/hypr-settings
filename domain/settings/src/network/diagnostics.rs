use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub level: LogLevel,
    pub text: String,
    /// Seconds since the epoch.
    pub unix_secs: u64,
}

/// The newest entries of what the app did, oldest dropped first.
#[derive(Debug, Clone)]
pub struct ActionLog {
    capacity: usize,
    entries: VecDeque<LogEntry>,
}

impl ActionLog {
    pub const DEFAULT_CAPACITY: usize = 200;

    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: VecDeque::new(),
        }
    }

    pub fn push(&mut self, level: LogLevel, text: impl Into<String>, unix_secs: u64) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(LogEntry {
            level,
            text: text.into(),
            unix_secs,
        });
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn entries(&self) -> Vec<LogEntry> {
        self.entries.iter().cloned().collect()
    }
}

impl Default for ActionLog {
    fn default() -> Self {
        Self::new(Self::DEFAULT_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_the_oldest_when_full() {
        let mut log = ActionLog::new(2);
        log.push(LogLevel::Info, "a", 1);
        log.push(LogLevel::Warn, "b", 2);
        log.push(LogLevel::Error, "c", 3);
        let texts: Vec<_> = log.entries().into_iter().map(|e| e.text).collect();
        assert_eq!(texts, ["b", "c"]);
        log.clear();
        assert!(log.entries().is_empty());
    }
}
