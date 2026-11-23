#[path = "interfaces/trial-pill-props.interface.rs"]
mod props;

#[path = "trial-pill.component.rs"]
mod component;
#[path = "trial-pill.html.rs"]
mod html;

pub use component::TrialPill;
pub use props::TrialPillProps;
