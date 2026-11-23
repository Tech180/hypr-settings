pub mod components;
pub mod constants;
pub mod interfaces;
pub mod utils;

#[path = "page.component.rs"]
mod component;
#[path = "page.html.rs"]
mod html;

pub use component::build_section;
