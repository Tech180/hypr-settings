use std::rc::Rc;

/// A 0-to-`max` value the user drags; `on_change` fires for every move, so the owner
/// debounces.
#[derive(Clone)]
pub struct SliderProps {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    /// Shown after the number, for example `%` or ` K`.
    pub unit: &'static str,
    pub on_change: Rc<dyn Fn(f64)>,
}
