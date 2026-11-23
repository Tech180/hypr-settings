use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ExecError(pub String);

/// Runs a program; the seam that keeps the controller testable.
pub trait Exec: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<(), ExecError>;
}

pub struct SystemExec;

impl Exec for SystemExec {
    fn run(&self, program: &str, args: &[&str]) -> Result<(), ExecError> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|e| ExecError(format!("{program}: {e}")))?;
        if output.status.success() {
            return Ok(());
        }
        Err(ExecError(format!(
            "{program} {} exited with {}: {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}
