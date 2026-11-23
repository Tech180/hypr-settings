//! Direct compositor apply through `hyprctl eval 'hl.monitor({...})'`. All of the
//! Hyprland Lua API surface for outputs is isolated in this file.

use std::sync::Arc;

use settings_domain::PortError;
use settings_domain::ports::{LiveOutputs, OutputConfig};

use crate::exec::{Exec, SystemExec};

pub struct HyprctlLiveOutputs {
    exec: Arc<dyn Exec>,
}

impl HyprctlLiveOutputs {
    pub fn new() -> Self {
        Self::with_exec(Arc::new(SystemExec))
    }

    pub fn with_exec(exec: Arc<dyn Exec>) -> Self {
        Self { exec }
    }
}

impl Default for HyprctlLiveOutputs {
    fn default() -> Self {
        Self::new()
    }
}

fn lua_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The explicit `transform` is always written so rotation can be reset.
pub fn monitor_snippet(config: &OutputConfig) -> String {
    let mut fields = vec![format!(
        "output = {}",
        lua_string(config.connector.as_str())
    )];
    if let Some(mode) = config.mode {
        fields.push(format!("mode = {}", lua_string(&mode.hypr_token())));
    }
    fields.push(format!(
        "position = {}",
        lua_string(&config.position.hypr_token())
    ));
    fields.push(format!("scale = {}", config.scale));
    fields.push(format!("disabled = {}", !config.enabled));
    fields.push(format!("transform = {}", config.transform.to_hypr()));
    format!("hl.monitor({{ {} }})", fields.join(", "))
}

pub fn eval_failed(output: &str) -> bool {
    let out = output.trim().to_lowercase();
    !(out.is_empty() || out == "ok")
        && ["error", "attempt to", "unknown"]
            .iter()
            .any(|needle| out.contains(needle))
}

impl LiveOutputs for HyprctlLiveOutputs {
    fn apply(&self, config: &OutputConfig) -> Result<(), PortError> {
        let snippet = monitor_snippet(config);
        let output = self
            .exec
            .run("hyprctl", &["eval", &snippet])
            .map_err(|e| PortError::Unavailable(e.to_string()))?;
        if eval_failed(&output) {
            return Err(PortError::Invalid(format!(
                "hyprctl eval rejected {snippet}: {}",
                output.trim()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use settings_domain::{ConnectorName, Mode, Position, Scale, Transform};

    use super::*;
    use crate::exec::ExecError;

    fn config(enabled: bool) -> OutputConfig {
        OutputConfig {
            connector: ConnectorName::try_from("DP-3").unwrap(),
            mode: Some("2560x1080@100".parse::<Mode>().unwrap()),
            position: Position::new(1920, 0),
            scale: Scale::new(1.25).unwrap(),
            transform: Transform::Normal,
            enabled,
            flags: Default::default(),
        }
    }

    #[test]
    fn builds_the_lua_call() {
        assert_eq!(
            monitor_snippet(&config(true)),
            r#"hl.monitor({ output = "DP-3", mode = "2560x1080@100.00", position = "1920x0", scale = 1.25, disabled = false, transform = 0 })"#
        );
        assert!(monitor_snippet(&config(false)).contains("disabled = true"));
    }

    #[test]
    fn escapes_lua_strings() {
        assert_eq!(lua_string(r#"a"b\c"#), r#""a\"b\\c""#);
    }

    #[test]
    fn recognises_failure_output() {
        assert!(!eval_failed(""));
        assert!(!eval_failed("ok\n"));
        assert!(eval_failed("[string] attempt to call a nil value"));
        assert!(eval_failed("Error: boom"));
    }

    struct Recorder(Mutex<Vec<String>>, &'static str);

    impl Exec for Recorder {
        fn run(&self, _program: &str, args: &[&str]) -> Result<String, ExecError> {
            self.0.lock().unwrap().push(args.join(" "));
            Ok(self.1.to_owned())
        }
    }

    #[test]
    fn apply_runs_hyprctl_eval_and_checks_the_reply() {
        let ok = Arc::new(Recorder(Mutex::default(), "ok"));
        HyprctlLiveOutputs::with_exec(ok.clone())
            .apply(&config(true))
            .unwrap();
        assert!(ok.0.lock().unwrap()[0].starts_with("eval hl.monitor("));

        let bad = Arc::new(Recorder(Mutex::default(), "error: nope"));
        assert!(matches!(
            HyprctlLiveOutputs::with_exec(bad).apply(&config(true)),
            Err(PortError::Invalid(_))
        ));
    }
}
