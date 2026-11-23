use std::rc::Rc;

#[derive(Clone)]
pub struct HzRateOption {
    pub hz: u32,
    pub active: bool,
    pub custom: bool,
}

#[derive(Clone)]
pub struct HzGroupProps {
    pub enabled: bool,
    pub current_hz: u32,
    pub options: Vec<HzRateOption>,
    pub on_pick: Rc<dyn Fn(u32)>,
    pub on_add_custom: Rc<dyn Fn(u32)>,
    pub on_remove_custom: Rc<dyn Fn(u32)>,
}
