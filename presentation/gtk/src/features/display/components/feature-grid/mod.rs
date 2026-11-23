#[path = "interfaces/feature-grid-props.interface.rs"]
mod props;

#[path = "feature-grid.component.rs"]
mod component;
#[path = "feature-grid.html.rs"]
mod html;

pub use component::build;
pub use props::{FeatureGridProps, FeatureTileProps};
