#[path = "interfaces/slider-props.interface.rs"]
mod props;

#[path = "slider.component.rs"]
mod component;
#[path = "slider.html.rs"]
mod html;

pub use component::build;
pub use props::SliderProps;
