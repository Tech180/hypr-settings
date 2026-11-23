use std::rc::Rc;
use std::sync::Arc;

use settings_application::SettingsApi;

/// What every section receives from the shell.
#[derive(Clone)]
pub struct ShellContext {
    pub api: Arc<dyn SettingsApi>,
    pub toast: Rc<dyn Fn(&str)>,
}
