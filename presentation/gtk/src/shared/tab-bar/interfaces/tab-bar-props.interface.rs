use std::rc::Rc;

use crate::shared::icon::Icon;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabProps {
    pub label: String,
    pub icon: Option<Icon>,
}

/// Page-level tabs with an accent underline on the active one.
#[derive(Clone)]
pub struct TabBarProps {
    pub tabs: Vec<TabProps>,
    pub selected: usize,
    pub on_select: Rc<dyn Fn(usize)>,
}
