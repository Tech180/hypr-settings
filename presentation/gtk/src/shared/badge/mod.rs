#[path = "interfaces/badge-props.interface.rs"]
mod props;

#[path = "badge.component.rs"]
mod component;
#[path = "badge.html.rs"]
mod html;

pub use component::{build, dot};
pub use props::{BadgeProps, Tone};
