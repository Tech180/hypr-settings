#[path = "interfaces/profile-detail-props.interface.rs"]
mod props;

#[path = "profile-detail.component.rs"]
mod component;
#[path = "profile-detail.html.rs"]
mod html;

pub use component::build;
pub use props::{ProfileDetailProps, SavedOutputProps};
