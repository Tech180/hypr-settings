//! `nmcli monitor`, debounced into "something changed" notifications.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use settings_domain::PortError;
use settings_domain::ports::{NetworkEvents, Subscription};

const DEBOUNCE: Duration = Duration::from_millis(400);
const POLL: Duration = Duration::from_millis(100);
const MAX_BACKOFF: Duration = Duration::from_secs(10);

type ChildSlot = Arc<Mutex<Option<Child>>>;

pub struct NmEvents {
    program: String,
    args: Vec<String>,
    debounce: Duration,
}

impl Default for NmEvents {
    fn default() -> Self {
        Self::new("nmcli", &["monitor"])
    }
}

impl NmEvents {
    pub fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            debounce: DEBOUNCE,
        }
    }

    pub fn with_debounce(mut self, debounce: Duration) -> Self {
        self.debounce = debounce;
        self
    }
}

/// Runs the monitor until it exits or `stop` is set; forwards one message per line.
fn read_monitor(
    program: &str,
    args: &[String],
    stop: &AtomicBool,
    slot: &ChildSlot,
    lines: &mpsc::Sender<()>,
) -> bool {
    let spawned = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(err) => {
            tracing::debug!(%err, "network monitor did not start");
            return false;
        }
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        return false;
    };
    *slot.lock().expect("child slot") = Some(child);
    // A cancel that raced the spawn must still stop this child.
    if stop.load(Ordering::Relaxed) {
        kill(slot);
    }

    let mut produced = false;
    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if !line.trim().is_empty() {
            produced = true;
            if lines.send(()).is_err() {
                break;
            }
        }
    }
    if let Some(mut child) = slot.lock().expect("child slot").take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    produced
}

fn kill(slot: &ChildSlot) {
    if let Some(child) = slot.lock().expect("child slot").as_mut() {
        let _ = child.kill();
    }
}

impl NetworkEvents for NmEvents {
    fn subscribe(&self, sink: Box<dyn Fn() + Send>) -> Result<Subscription, PortError> {
        let stop = Arc::new(AtomicBool::new(false));
        let slot: ChildSlot = Arc::new(Mutex::new(None));
        let (tx, rx) = mpsc::channel::<()>();

        {
            let (stop, slot) = (stop.clone(), slot.clone());
            let (program, args) = (self.program.clone(), self.args.clone());
            thread::Builder::new()
                .name("network-monitor".into())
                .spawn(move || {
                    let mut backoff = Duration::from_millis(500);
                    while !stop.load(Ordering::Relaxed) {
                        if read_monitor(&program, &args, &stop, &slot, &tx) {
                            backoff = Duration::from_millis(500);
                        }
                        let until = Instant::now() + backoff;
                        while Instant::now() < until && !stop.load(Ordering::Relaxed) {
                            thread::sleep(POLL);
                        }
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                    }
                })
                .map_err(|e| PortError::Io(e.to_string()))?;
        }

        let debounce = self.debounce;
        let stop_dispatch = stop.clone();
        thread::Builder::new()
            .name("network-events".into())
            .spawn(move || {
                let mut pending: Option<Instant> = None;
                while !stop_dispatch.load(Ordering::Relaxed) {
                    match rx.recv_timeout(POLL) {
                        // The first event of a burst starts the clock, so a chatty monitor cannot
                        // postpone the notification forever.
                        Ok(()) => {
                            pending.get_or_insert_with(Instant::now);
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                    if pending.is_some_and(|at| at.elapsed() >= debounce) {
                        pending = None;
                        sink();
                    }
                }
            })
            .map_err(|e| PortError::Io(e.to_string()))?;

        Ok(Subscription::new(move || {
            stop.store(true, Ordering::Relaxed);
            kill(&slot);
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(script: &str) -> NmEvents {
        NmEvents::new("sh", &["-c", script]).with_debounce(Duration::from_millis(150))
    }

    #[test]
    fn a_burst_of_lines_is_one_notification() {
        let (tx, rx) = mpsc::channel();
        let _sub = events("echo one; echo two; echo three; sleep 30")
            .subscribe(Box::new(move || {
                let _ = tx.send(());
            }))
            .unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(3)).is_ok());
        assert!(rx.recv_timeout(Duration::from_millis(500)).is_err());
    }

    #[test]
    fn cancelling_stops_the_notifications_and_the_child() {
        let (tx, rx) = mpsc::channel();
        let sub = events("while true; do echo tick; sleep 0.05; done")
            .subscribe(Box::new(move || {
                let _ = tx.send(());
            }))
            .unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(3)).is_ok());
        drop(sub);
        while rx.recv_timeout(Duration::from_millis(400)).is_ok() {}
        assert!(rx.recv_timeout(Duration::from_millis(600)).is_err());
    }

    #[test]
    fn a_missing_program_is_retried_quietly() {
        let events = NmEvents::new("definitely-not-a-program", &[]);
        let sub = events.subscribe(Box::new(|| {})).unwrap();
        drop(sub);
    }
}
