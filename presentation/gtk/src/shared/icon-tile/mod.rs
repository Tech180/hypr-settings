#[path = "interfaces/icon-tile-props.interface.rs"]
mod props;

#[path = "icon-tile.component.rs"]
mod component;
#[path = "icon-tile.html.rs"]
mod html;

pub use component::build;
pub use props::IconTileProps;
