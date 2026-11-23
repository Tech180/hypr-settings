//! Hyprland `.socket2.sock` event stream, debounced into `TopologyEvent::Changed`.

use std::io::{BufRead, BufReader, ErrorKind};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use settings_domain::PortError;
use settings_domain::ports::{Subscription, TopologyEvent, TopologyEvents};

const DEBOUNCE: Duration = Duration::from_millis(300);
const POLL: Duration = Duration::from_millis(100);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

pub struct HyprEvents {
    socket: PathBuf,
    debounce: Duration,
}

impl HyprEvents {
    pub fn new(socket: PathBuf) -> Self {
        Self {
            socket,
            debounce: DEBOUNCE,
        }
    }

    pub fn with_debounce(mut self, debounce: Duration) -> Self {
        self.debounce = debounce;
        self
    }

    /// `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock`
    pub fn from_env() -> Result<Self, PortError> {
        let runtime = std::env::var("XDG_RUNTIME_DIR")
            .map_err(|_| PortError::Unavailable("XDG_RUNTIME_DIR is not set".into()))?;
        let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .map_err(|_| PortError::Unavailable("HYPRLAND_INSTANCE_SIGNATURE is not set".into()))?;
        Ok(Self::new(
            PathBuf::from(runtime)
                .join("hypr")
                .join(signature)
                .join(".socket2.sock"),
        ))
    }
}

/// Events that can change the set or shape of outputs (`name>>data` lines).
pub fn is_relevant(line: &str) -> bool {
    let name = line.split_once(">>").map_or(line, |(name, _)| name);
    matches!(
        name.trim(),
        "monitoradded"
            | "monitoraddedv2"
            | "monitorremoved"
            | "monitorremovedv2"
            | "configreloaded"
    )
}

/// Reads one connection until EOF or `stop`. Returns false when `stop` was set.
fn pump(
    stream: UnixStream,
    stop: &AtomicBool,
    debounce: Duration,
    sink: &(dyn Fn(TopologyEvent) + Send),
    pending: &mut Option<Instant>,
) -> bool {
    if stream.set_read_timeout(Some(POLL)).is_err() {
        return true;
    }
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    while !stop.load(Ordering::Relaxed) {
        match reader.read_line(&mut line) {
            Ok(0) => return true,
            Ok(_) if line.ends_with('\n') => {
                if is_relevant(line.trim_end()) {
                    *pending = Some(Instant::now());
                }
                line.clear();
            }
            Ok(_) => {}
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(_) => return true,
        }
        if pending.is_some_and(|at| at.elapsed() >= debounce) {
            *pending = None;
            sink(TopologyEvent::Changed);
        }
    }
    false
}

impl TopologyEvents for HyprEvents {
    fn subscribe(
        &self,
        sink: Box<dyn Fn(TopologyEvent) + Send>,
    ) -> Result<Subscription, PortError> {
        if !self.socket.exists() {
            return Err(PortError::Unavailable(format!(
                "{} does not exist",
                self.socket.display()
            )));
        }
        let socket = self.socket.clone();
        let debounce = self.debounce;
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();

        thread::Builder::new()
            .name("hypr-events".into())
            .spawn(move || {
                let mut backoff = Duration::from_millis(500);
                let mut pending = None;
                while !stop_thread.load(Ordering::Relaxed) {
                    match UnixStream::connect(&socket) {
                        Ok(stream) => {
                            backoff = Duration::from_millis(500);
                            if !pump(stream, &stop_thread, debounce, sink.as_ref(), &mut pending) {
                                return;
                            }
                        }
                        Err(err) => tracing::debug!(%err, "hyprland event socket unavailable"),
                    }
                    // Reconnect with backoff, staying responsive to cancellation.
                    let until = Instant::now() + backoff;
                    while Instant::now() < until && !stop_thread.load(Ordering::Relaxed) {
                        thread::sleep(POLL);
                    }
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                }
            })
            .map_err(|e| PortError::Io(e.to_string()))?;

        Ok(Subscription::new(move || {
            stop.store(true, Ordering::Relaxed)
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::os::unix::net::UnixListener;
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn only_output_events_are_relevant() {
        assert!(is_relevant("monitoradded>>DP-3"));
        assert!(is_relevant("monitorremovedv2>>1,DP-3,desc"));
        assert!(is_relevant("configreloaded>>"));
        assert!(!is_relevant("workspace>>2"));
        assert!(!is_relevant("activewindow>>kitty,title"));
    }

    fn socket_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("hypr-settings-{tag}-{}.sock", std::process::id()))
    }

    #[test]
    fn bursts_collapse_into_one_notification_and_survive_reconnects() {
        let path = socket_path("events");
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();

        let (tx, rx) = mpsc::channel();
        let events = HyprEvents::new(path.clone()).with_debounce(Duration::from_millis(150));
        let _sub = events
            .subscribe(Box::new(move |e| {
                let _ = tx.send(e);
            }))
            .unwrap();

        let (mut first, _) = listener.accept().unwrap();
        first
            .write_all(b"monitoradded>>DP-3\nworkspace>>2\nmonitoraddedv2>>1,DP-3,x\n")
            .unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(3)),
            Ok(TopologyEvent::Changed)
        );
        assert!(rx.recv_timeout(Duration::from_millis(400)).is_err());

        drop(first);
        let (mut second, _) = listener.accept().unwrap();
        second.write_all(b"monitorremoved>>DP-3\n").unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(3)),
            Ok(TopologyEvent::Changed)
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_socket_is_reported() {
        let events = HyprEvents::new(socket_path("missing"));
        assert!(matches!(
            events.subscribe(Box::new(|_| {})),
            Err(PortError::Unavailable(_))
        ));
    }
}
