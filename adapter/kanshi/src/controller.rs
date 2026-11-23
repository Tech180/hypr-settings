use std::sync::Arc;

use settings_domain::ports::ProfileController;
use settings_domain::{PortError, ProfileName};

use crate::exec::{Exec, SystemExec};

pub struct Kanshictl {
    exec: Arc<dyn Exec>,
}

impl Kanshictl {
    pub fn new() -> Self {
        Self::with_exec(Arc::new(SystemExec))
    }

    pub fn with_exec(exec: Arc<dyn Exec>) -> Self {
        Self { exec }
    }
}

impl Default for Kanshictl {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileController for Kanshictl {
    /// Re-reads the conf in place; falls back to the systemd unit, then a restart.
    fn reload(&self) -> Result<(), PortError> {
        let ladder: [(&str, &[&str]); 3] = [
            ("kanshictl", &["reload"]),
            ("systemctl", &["--user", "reload", "kanshi"]),
            ("systemctl", &["--user", "restart", "kanshi"]),
        ];
        let mut last = String::new();
        for (program, args) in ladder {
            match self.exec.run(program, args) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    tracing::warn!(%err, "kanshi reload step failed");
                    last = err.to_string();
                }
            }
        }
        Err(PortError::Unavailable(format!(
            "kanshi did not reload: {last}"
        )))
    }

    fn switch(&self, name: &ProfileName) -> Result<(), PortError> {
        self.exec
            .run("kanshictl", &["switch", name.as_str()])
            .map_err(|e| PortError::Unavailable(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::exec::ExecError;

    /// Fails every command whose joined form is in `failing`.
    struct Scripted {
        failing: Vec<&'static str>,
        calls: Mutex<Vec<String>>,
    }

    impl Exec for Scripted {
        fn run(&self, program: &str, args: &[&str]) -> Result<(), ExecError> {
            let call = format!("{program} {}", args.join(" "));
            self.calls.lock().unwrap().push(call.clone());
            if self.failing.contains(&call.as_str()) {
                Err(ExecError(format!("{call} failed")))
            } else {
                Ok(())
            }
        }
    }

    fn controller(failing: Vec<&'static str>) -> (Kanshictl, Arc<Scripted>) {
        let exec = Arc::new(Scripted {
            failing,
            calls: Mutex::default(),
        });
        (Kanshictl::with_exec(exec.clone()), exec)
    }

    #[test]
    fn reload_prefers_kanshictl() {
        let (ctl, exec) = controller(vec![]);
        ctl.reload().unwrap();
        assert_eq!(*exec.calls.lock().unwrap(), ["kanshictl reload"]);
    }

    #[test]
    fn reload_walks_the_fallback_ladder() {
        let (ctl, exec) = controller(vec!["kanshictl reload", "systemctl --user reload kanshi"]);
        ctl.reload().unwrap();
        assert_eq!(exec.calls.lock().unwrap().len(), 3);
    }

    #[test]
    fn reload_fails_when_every_step_fails() {
        let (ctl, _) = controller(vec![
            "kanshictl reload",
            "systemctl --user reload kanshi",
            "systemctl --user restart kanshi",
        ]);
        assert!(matches!(ctl.reload(), Err(PortError::Unavailable(_))));
    }

    #[test]
    fn switch_passes_the_validated_name() {
        let (ctl, exec) = controller(vec![]);
        ctl.switch(&ProfileName::parse("docked").unwrap()).unwrap();
        assert_eq!(*exec.calls.lock().unwrap(), ["kanshictl switch docked"]);
    }
}
