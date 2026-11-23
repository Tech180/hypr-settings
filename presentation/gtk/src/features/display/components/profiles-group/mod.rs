#[path = "interfaces/profiles-group-props.interface.rs"]
mod props;

#[path = "profiles-group.component.rs"]
mod component;
#[path = "profiles-group.html.rs"]
mod html;

pub use component::build;
pub use props::{ProfileRowProps, ProfilesGroupProps};
