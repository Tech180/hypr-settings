/// What the process was asked to do with the settings window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    Open { section: Option<String> },
    Toggle,
    Close,
    Refresh,
}
