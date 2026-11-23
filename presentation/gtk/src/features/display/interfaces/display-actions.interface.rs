use std::rc::Rc;

use settings_domain::controls::Flag;

/// What the user can ask of the display page; the page component implements each one.
#[derive(Clone)]
pub struct DisplayActions {
    pub select_output: Rc<dyn Fn(String)>,
    /// Drop of a tile on the arrangement canvas: connector and new layout position.
    pub move_output: Rc<dyn Fn(String, i32, i32)>,
    pub set_resolution: Rc<dyn Fn(u32, u32)>,
    pub set_refresh: Rc<dyn Fn(u32)>,
    pub set_scale: Rc<dyn Fn(u16)>,
    pub set_transform: Rc<dyn Fn(i32)>,
    pub set_enabled: Rc<dyn Fn(bool)>,
    pub open_profile: Rc<dyn Fn(String)>,
    pub close_profile: Rc<dyn Fn()>,
    pub apply_profile: Rc<dyn Fn(String)>,
    pub delete_profile: Rc<dyn Fn(String)>,
    pub save_profile_as: Rc<dyn Fn(String)>,
    /// Stores the live layout in the active profile.
    pub save_active: Rc<dyn Fn()>,
    /// Makes the selected output the one that hosts the bar.
    pub set_main: Rc<dyn Fn()>,
    pub add_custom_hz: Rc<dyn Fn(u32)>,
    pub remove_custom_hz: Rc<dyn Fn(u32)>,
    /// Sliders report every move; the page debounces them.
    pub set_brightness: Rc<dyn Fn(f64)>,
    pub set_sync: Rc<dyn Fn(bool)>,
    pub set_contrast: Rc<dyn Fn(f64)>,
    pub set_overdrive: Rc<dyn Fn(f64)>,
    /// The VCP value of the input source to switch the selected output to.
    pub set_input: Rc<dyn Fn(u8)>,
    pub set_flag: Rc<dyn Fn(Flag, bool)>,
    pub set_night: Rc<dyn Fn(bool)>,
    pub set_kelvin: Rc<dyn Fn(f64)>,
}
