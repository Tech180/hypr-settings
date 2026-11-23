/// How loudly a badge or status dot speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    Neutral,
    Accent,
    Warning,
    Danger,
}

impl Tone {
    pub(crate) const fn suffix(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Accent => "accent",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }
}

/// A small rounded pill carrying a short fact, such as a battery level or a spec version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadgeProps {
    pub text: String,
    pub tone: Tone,
}

impl BadgeProps {
    pub fn new(text: impl Into<String>, tone: Tone) -> Self {
        Self {
            text: text.into(),
            tone,
        }
    }
}
