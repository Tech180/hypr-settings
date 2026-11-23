pub mod components;
pub mod constants;
pub mod interfaces;
pub mod utils;

#[path = "page.html.rs"]
mod html;

pub use components::page_entry::build_section;
