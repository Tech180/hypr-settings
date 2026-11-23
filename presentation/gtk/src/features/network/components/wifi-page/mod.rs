#[path = "wifi-page.html.rs"]
mod html;

pub use html::{build, refresh_wifi_lists, WifiListSurfaces};
