//! A fixed colour temperature through `hyprsunset`. The systemd user service runs the
//! schedule; a manual setting stops it and runs `hyprsunset --temperature K` instead.

use std::sync::Arc;
use std::time::Duration;

use settings_domain::ports::NightLight;
use settings_domain::{Kelvin, PortError};

use crate::exec::{Exec, SystemExec};

const SERVICE: &str = "hyprsunset.service";
/// The colour transform is released a moment after the old process ends.
const RELEASE_DELAY: Duration = Duration::from_millis(150);

pub struct HyprsunsetNightLight {
    exec: Arc<dyn Exec>,
    release_delay: Duration,
}

impl HyprsunsetNightLight {
    pub fn new() -> Self {
        Self::with_exec(Arc::new(SystemExec), RELEASE_DELAY)
    }

    pub fn with_exec(exec: Arc<dyn Exec>, release_delay: Duration) -> Self {
        Self {
            exec,
            release_delay,
        }
    }

    /// Stops whatever drives the screen colour: the service and any fixed-temperature run.
    fn stop_all(&self) {
        // Each may legitimately fail (not running), so failures are not interesting.
        let _ = self.exec.run("systemctl", &["--user", "stop", SERVICE]);
        let _ = self.exec.run("pkill", &["-x", "hyprsunset"]);
    }
}

impl Default for HyprsunsetNightLight {
    fn default() -> Self {
        Self::new()
    }
}

impl NightLight for HyprsunsetNightLight {
    fn enable(&self, kelvin: Kelvin) -> Result<(), PortError> {
        self.stop_all();
        std::thread::sleep(self.release_delay);
        let temperature = kelvin.get().to_string();
        self.exec
            .spawn_detached("hyprsunset", &["--temperature", &temperature])
            .map_err(|e| PortError::Unavailable(e.to_string()))
    }

    fn disable(&self) -> Result<(), PortError> {
        let _ = self.exec.run("pkill", &["-x", "hyprsunset"]);
        self.exec
            .run("systemctl", &["--user", "start", SERVICE])
            .map(|_| ())
            .map_err(|e| PortError::Unavailable(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::exec::ExecError;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);

    impl Exec for Recorder {
        fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError> {
            self.0
                .lock()
                .unwrap()
                .push(format!("run {program} {}", args.join(" ")));
            if program == "pkill" {
                return Err(ExecError("nothing to kill".into()));
            }
            Ok(String::new())
        }

        fn spawn_detached(&self, program: &str, args: &[&str]) -> Result<(), ExecError> {
            self.0
                .lock()
                .unwrap()
                .push(format!("spawn {program} {}", args.join(" ")));
            Ok(())
        }
    }

    #[test]
    fn enabling_replaces_the_schedule_with_a_fixed_temperature() {
        let exec = Arc::new(Recorder::default());
        let light = HyprsunsetNightLight::with_exec(exec.clone(), Duration::ZERO);
        light.enable(Kelvin::clamped(4200)).unwrap();
        assert_eq!(
            *exec.0.lock().unwrap(),
            [
                "run systemctl --user stop hyprsunset.service",
                "run pkill -x hyprsunset",
                "spawn hyprsunset --temperature 4200",
            ]
        );
    }

    #[test]
    fn disabling_brings_the_schedule_back_even_when_nothing_was_running() {
        let exec = Arc::new(Recorder::default());
        let light = HyprsunsetNightLight::with_exec(exec.clone(), Duration::ZERO);
        light.disable().unwrap();
        assert_eq!(
            *exec.0.lock().unwrap(),
            [
                "run pkill -x hyprsunset",
                "run systemctl --user start hyprsunset.service",
            ]
        );
    }
}
