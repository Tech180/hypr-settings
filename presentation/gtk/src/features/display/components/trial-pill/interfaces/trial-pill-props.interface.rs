use std::rc::Rc;

use gtk4 as gtk;

#[derive(Clone)]
pub struct TrialPillProps {
    pub on_keep: Rc<dyn Fn()>,
    pub on_revert: Rc<dyn Fn()>,
}

pub struct TrialPillWidgets {
    pub root: gtk::Revealer,
    pub message: gtk::Label,
    pub keep: gtk::Button,
    pub revert: gtk::Button,
}
