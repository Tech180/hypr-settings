#[path = "interfaces/settings-group-props.interface.rs"]
mod props;

#[path = "settings-group.component.rs"]
mod component;
#[path = "settings-group.html.rs"]
mod html;

pub use component::build;
pub use props::{RowControl, SettingRowProps, SettingsGroupProps, SliderProps};
