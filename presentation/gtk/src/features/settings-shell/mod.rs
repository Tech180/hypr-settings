pub mod components;
pub mod constants;
pub mod interfaces;
pub mod utils;

#[path = "shell.component.rs"]
mod component;
#[path = "shell.html.rs"]
mod html;

pub use component::ShellWindow;
