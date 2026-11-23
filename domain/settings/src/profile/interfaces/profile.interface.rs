use serde::{Deserialize, Serialize};

use crate::value::{Mode, Position, Scale, Transform};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProfileName(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Criteria {
    Named(String),
    Any,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileOutput {
    pub criteria: Criteria,
    /// `None` means the conf did not say; kanshi then leaves the status alone.
    pub enable: Option<bool>,
    pub mode: Option<Mode>,
    pub position: Option<Position>,
    pub scale: Option<Scale>,
    pub transform: Option<Transform>,
    /// Kanshi `...output`: applies to every remaining output.
    pub rest_glob: bool,
    /// Tokens the model does not understand, kept verbatim so rewrites are lossless.
    pub extra: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: ProfileName,
    pub outputs: Vec<ProfileOutput>,
    /// Non-output lines inside the block (for example `exec`), preserved verbatim.
    pub extra_lines: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileSet {
    pub profiles: Vec<Profile>,
    /// Top-level lines outside any profile (for example `include`), preserved verbatim.
    pub preamble: Vec<String>,
}

/// Where a layout edit ended up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertOutcome {
    /// False when the change cannot be represented in a profile.
    pub stored: bool,
    /// The profile that now holds the layout: the active one, or a new specific
    /// profile when the active profile is a catch-all.
    pub profile: ProfileName,
}
