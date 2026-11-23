use crate::output::constants::ULTRAWIDE_ASPECT;
use crate::output::interfaces::{DisplayKind, Output, OutputIdentity, Topology};
use crate::output::utils::logical_size;
use crate::value::{ConnectorName, Mode};

impl DisplayKind {
    pub fn infer(connector: &ConnectorName, description: &str) -> Self {
        if connector.is_internal() {
            return Self::Laptop;
        }
        let is_tv = description
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| matches!(w, "oled" | "tv" | "television"));
        if is_tv { Self::Tv } else { Self::Monitor }
    }
}

impl Output {
    pub fn is_ultrawide(&self) -> bool {
        self.mode.is_some_and(|m| {
            let r = m.resolution;
            !r.is_empty() && f64::from(r.width) / f64::from(r.height) >= ULTRAWIDE_ASPECT
        })
    }

    /// Size in the compositor's layout space: the mode divided by the scale and
    /// rounded, with width and height swapped for quarter-turn transforms.
    /// `None` while the output has no mode.
    pub fn logical_size(&self) -> Option<(i32, i32)> {
        let mode = self.mode?;
        Some(logical_size(mode.resolution, self.scale, self.transform))
    }

    /// Mode to persist when nothing else was requested: the advertised mode at the
    /// current rounded rate, falling back to the live mode.
    pub fn persisted_mode(&self) -> Option<Mode> {
        let current = self.mode?;
        Some(
            self.modes
                .exact_for(current.resolution, current.refresh.rounded_hz())
                .unwrap_or(current),
        )
    }
}

/// What a kanshi profile can recognise a connected display by.
impl OutputIdentity {
    pub fn of(output: &Output) -> Self {
        Self {
            connector: output.connector.clone(),
            make: output.make.clone(),
            model: output.model.clone(),
            serial: output.serial.clone(),
        }
    }

    /// `Make Model Serial` the way kanshi spells it, with `Unknown` for blanks.
    pub fn description(&self) -> String {
        let field = |v: &str| if v.is_empty() { "Unknown" } else { v }.to_owned();
        format!(
            "{} {} {}",
            field(&self.make),
            field(&self.model),
            field(&self.serial)
        )
    }

    /// The spellings kanshi may compare against: blanks as `Unknown`, and blanks
    /// left empty (compositors differ on whether an unset serial is sent).
    pub fn descriptions(&self) -> [String; 2] {
        [
            self.description(),
            format!("{} {} {}", self.make, self.model, self.serial),
        ]
    }

    /// False when the display reports nothing that tells it apart.
    pub fn has_description(&self) -> bool {
        !(self.make.is_empty() && self.model.is_empty() && self.serial.is_empty())
    }
}

impl Topology {
    pub fn new(outputs: Vec<Output>) -> Self {
        Self(outputs)
    }

    pub fn outputs(&self) -> &[Output] {
        &self.0
    }

    pub fn find(&self, connector: &ConnectorName) -> Option<&Output> {
        self.0.iter().find(|o| &o.connector == connector)
    }

    pub fn connected_names(&self) -> Vec<ConnectorName> {
        self.0.iter().map(|o| o.connector.clone()).collect()
    }

    pub fn identities(&self) -> Vec<OutputIdentity> {
        self.0.iter().map(OutputIdentity::of).collect()
    }

    pub fn enabled_count(&self) -> usize {
        self.0.iter().filter(|o| o.enabled).count()
    }

    /// The display that hosts the bar: the preferred one while it is on, otherwise the
    /// laptop panel, otherwise the first display that is on.
    pub fn bar_host(&self, preferred: Option<&str>) -> Option<&Output> {
        let mut on = self.0.iter().filter(|o| o.enabled);
        if let Some(found) = preferred.and_then(|p| on.clone().find(|o| o.connector.as_str() == p))
        {
            return Some(found);
        }
        on.clone()
            .find(|o| o.kind == DisplayKind::Laptop)
            .or_else(|| on.next())
    }
}
