use std::rc::Rc;

pub struct SectionSearchProps {
    pub on_change: Rc<dyn Fn(String)>,
    pub on_activate: Rc<dyn Fn()>,
    /// Escape pressed with an empty query.
    pub on_dismiss: Rc<dyn Fn()>,
}
