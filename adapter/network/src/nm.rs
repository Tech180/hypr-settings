use std::sync::Arc;

use settings_domain::PortError;

use crate::exec::{Exec, ExecError};
use crate::parse::Show;

/// `nmcli` through an [`Exec`]. Every argument is its own array element.
#[derive(Clone)]
pub struct Nm {
    exec: Arc<dyn Exec>,
}

impl Nm {
    pub fn new(exec: Arc<dyn Exec>) -> Self {
        Self { exec }
    }

    pub fn run(&self, args: &[&str]) -> Result<String, ExecError> {
        self.exec.run("nmcli", args)
    }

    pub fn run_owned(&self, args: &[String]) -> Result<String, ExecError> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&refs)
    }

    /// `nmcli connection modify id <id> <settings...>`.
    pub fn modify(&self, id: &str, settings: &[String]) -> Result<(), PortError> {
        let mut args = vec![
            "connection".to_owned(),
            "modify".into(),
            "id".into(),
            id.to_owned(),
        ];
        args.extend(settings.iter().cloned());
        self.run_owned(&args).map(drop).map_err(port)
    }

    pub fn show(&self, args: &[&str]) -> Option<Show> {
        self.run(args).ok().map(|out| Show::parse(&out))
    }

    pub fn device_show(&self, iface: &str) -> Option<Show> {
        self.show(&["-t", "-f", "all", "device", "show", iface])
    }

    pub fn connection_show(&self, id: &str) -> Option<Show> {
        self.show(&["-t", "connection", "show", "id", id])
    }

    pub fn connection_secrets(&self, id: &str) -> Option<Show> {
        self.show(&["--show-secrets", "-t", "connection", "show", "id", id])
    }
}

pub fn port(err: ExecError) -> PortError {
    PortError::Unavailable(err.0)
}
