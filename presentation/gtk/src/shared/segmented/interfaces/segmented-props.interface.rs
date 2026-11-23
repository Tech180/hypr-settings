use std::rc::Rc;

#[derive(Clone)]
pub struct SegmentedProps {
    pub labels: Vec<String>,
    pub selected: Option<usize>,
    /// Called with the index of any button that is pressed.
    pub on_select: Rc<dyn Fn(usize)>,
}
