use std::rc::Rc;

#[derive(Clone)]
pub struct DropdownProps {
    pub options: Vec<String>,
    pub selected: usize,
    pub on_select: Rc<dyn Fn(usize)>,
}
