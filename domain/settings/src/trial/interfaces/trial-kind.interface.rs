#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialKind {
    Resolution,
    Refresh,
    Scale,
    Transform,
    Position,
}
