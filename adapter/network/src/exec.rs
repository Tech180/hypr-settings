use std::process::{Command, Stdio};

/// A failed command, carrying what the tool said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecError(pub String);

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Runs a program with an argument array (never a shell string) and returns stdout.
/// The seam that keeps the nmcli and systemctl code testable.
pub trait Exec: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError>;
}

/// Real processes, with `LC_ALL=C` so output stays parseable in any locale.
pub struct SystemExec;

impl Exec for SystemExec {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError> {
        let output = Command::new(program)
            .args(args)
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .output()
            .map_err(|e| ExecError(format!("{program}: {e}")))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        Err(ExecError(detail.to_owned()))
    }
}
