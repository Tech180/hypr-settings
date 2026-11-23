#[path = "interfaces/arrangement-canvas-props.interface.rs"]
mod props;

#[path = "arrangement-canvas.component.rs"]
mod component;
#[path = "arrangement-canvas.html.rs"]
mod html;

pub use component::ArrangementCanvas;
pub use props::{ArrangementCanvasProps, PreviewDrop, TileProps};
