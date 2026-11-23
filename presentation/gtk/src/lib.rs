//! GTK4 / libadwaita driving adapter: the settings window and its single-instance app.

mod app;
mod features;
mod shared;
mod style;

pub use app::interfaces::Launch;
pub use app::run;
