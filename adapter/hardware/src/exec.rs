use std::process::{Command, Stdio};

use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ExecError(pub String);

/// Runs programs; the seam that keeps these adapters testable.
pub trait Exec: Send + Sync {
    /// Runs to completion and returns stdout.
    fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError>;
    /// Starts a program that outlives this process.
    fn spawn_detached(&self, program: &str, args: &[&str]) -> Result<(), ExecError>;
}

pub struct SystemExec;

impl Exec for SystemExec {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError> {
        let output = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| ExecError(format!("{program}: {e}")))?;
        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ExecError(format!(
                "{program} {} exited with {}: {}{}",
                args.join(" "),
                output.status,
                stderr.trim(),
                stdout.trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn spawn_detached(&self, program: &str, args: &[&str]) -> Result<(), ExecError> {
        use std::os::unix::process::CommandExt;
        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0);
        let mut child = command
            .spawn()
            .map_err(|e| ExecError(format!("{program}: {e}")))?;
        // Reaped in the background so it never lingers as a zombie while we run.
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }
}
