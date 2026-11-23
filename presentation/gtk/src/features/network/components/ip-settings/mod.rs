#[path = "interfaces/ip-settings-props.interface.rs"]
mod props;
#[path = "ip-settings.html.rs"]
mod html;

pub use html::build;
pub use props::IpSettingsProps;
