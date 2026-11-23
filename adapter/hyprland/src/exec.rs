use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ExecError(pub String);

/// Runs a program and returns its stdout; the seam that keeps adapters testable.
pub trait Exec: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError>;
}

pub struct SystemExec;

impl Exec for SystemExec {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|e| ExecError(format!("{program}: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ExecError(format!(
                "{program} {} exited with {}: {}",
                args.join(" "),
                output.status,
                stderr.trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}
