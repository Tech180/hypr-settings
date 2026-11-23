use std::rc::Rc;

#[derive(Clone)]
pub struct ToggleProps {
    pub active: bool,
    pub on_toggle: Rc<dyn Fn(bool)>,
}
