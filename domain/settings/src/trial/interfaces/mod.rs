#[path = "trial.interface.rs"]
mod trial;
#[path = "trial-kind.interface.rs"]
mod trial_kind;
#[path = "trial-policy.interface.rs"]
mod trial_policy;
#[path = "trial-slot.interface.rs"]
mod trial_slot;

pub use trial::Trial;
pub use trial_kind::TrialKind;
pub use trial_policy::TrialPolicy;
pub use trial_slot::TrialSlot;
