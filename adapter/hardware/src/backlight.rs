use std::sync::Arc;

use settings_domain::PortError;
use settings_domain::controls::clamp_percent;
use settings_domain::ports::Backlight;

use crate::exec::{Exec, SystemExec};

pub struct BrightnessctlBacklight {
    exec: Arc<dyn Exec>,
}

impl BrightnessctlBacklight {
    pub fn new() -> Self {
        Self::with_exec(Arc::new(SystemExec))
    }

    pub fn with_exec(exec: Arc<dyn Exec>) -> Self {
        Self { exec }
    }
}

impl Default for BrightnessctlBacklight {
    fn default() -> Self {
        Self::new()
    }
}

/// `brightnessctl -m`: `device,class,current,percent%,max`.
fn parse_machine(output: &str) -> Option<u8> {
    let percent = output.trim().lines().next()?.split(',').nth(3)?;
    let value: i64 = percent.trim().trim_end_matches('%').parse().ok()?;
    Some(clamp_percent(value))
}

impl Backlight for BrightnessctlBacklight {
    fn read(&self) -> Result<Option<u8>, PortError> {
        // No backlight device is not an error: desktops have none.
        Ok(self
            .exec
            .run("brightnessctl", &["-m"])
            .ok()
            .and_then(|out| parse_machine(&out)))
    }

    fn set(&self, percent: u8) -> Result<(), PortError> {
        let value = format!("{}%", percent.min(100));
        self.exec
            .run("brightnessctl", &["set", &value])
            .map(|_| ())
            .map_err(|e| PortError::Unavailable(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::exec::ExecError;

    struct Scripted(Mutex<Vec<String>>, Result<&'static str, &'static str>);

    impl Exec for Scripted {
        fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError> {
            self.0
                .lock()
                .unwrap()
                .push(format!("{program} {}", args.join(" ")));
            self.1
                .map(str::to_owned)
                .map_err(|e| ExecError(e.to_owned()))
        }

        fn spawn_detached(&self, _: &str, _: &[&str]) -> Result<(), ExecError> {
            unreachable!()
        }
    }

    #[test]
    fn reads_the_percentage_column() {
        assert_eq!(
            parse_machine("intel_backlight,backlight,9600,40%,24000\n"),
            Some(40)
        );
        assert_eq!(parse_machine("x,backlight,1,250%,1"), Some(100));
        assert_eq!(parse_machine("garbage"), None);
        assert_eq!(parse_machine(""), None);
    }

    #[test]
    fn a_machine_without_backlight_reads_as_none() {
        let backlight = BrightnessctlBacklight::with_exec(Arc::new(Scripted(
            Mutex::default(),
            Err("no devices"),
        )));
        assert_eq!(backlight.read().unwrap(), None);
        assert!(backlight.set(10).is_err());
    }

    #[test]
    fn set_writes_a_percentage() {
        let exec = Arc::new(Scripted(Mutex::default(), Ok("")));
        BrightnessctlBacklight::with_exec(exec.clone())
            .set(150)
            .unwrap();
        assert_eq!(exec.0.lock().unwrap()[0], "brightnessctl set 100%");
    }
}
