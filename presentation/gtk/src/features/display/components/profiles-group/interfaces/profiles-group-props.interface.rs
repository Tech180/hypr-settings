use std::rc::Rc;

use crate::shared::icon::Icon;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRowProps {
    pub name: String,
    pub subtitle: String,
    pub active: bool,
    pub icon: Icon,
}

#[derive(Clone)]
pub struct ProfilesGroupProps {
    pub rows: Vec<ProfileRowProps>,
    pub on_open: Rc<dyn Fn(usize)>,
    pub on_save_as: Rc<dyn Fn(String)>,
    pub active_name: Option<String>,
    pub on_save_active: Rc<dyn Fn()>,
}
