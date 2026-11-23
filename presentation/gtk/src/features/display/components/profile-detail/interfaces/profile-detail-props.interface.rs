use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedOutputProps {
    pub title: String,
    pub subtitle: String,
    /// Where the profile places the output, such as `+1920, 0`.
    pub badge: Option<String>,
}

#[derive(Clone)]
pub struct ProfileDetailProps {
    pub name: String,
    pub subtitle: String,
    pub outputs: Vec<SavedOutputProps>,
    pub removable: bool,
    pub on_back: Rc<dyn Fn()>,
    pub on_delete: Rc<dyn Fn()>,
    pub on_apply: Rc<dyn Fn()>,
}
