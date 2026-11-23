use std::rc::Rc;

use crate::shared::icon::Icon;

#[derive(Clone)]
pub struct FeatureTileProps {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub icon: Icon,
    pub active: bool,
    pub sensitive: bool,
    pub tooltip: Option<String>,
    pub on_toggle: Rc<dyn Fn(bool)>,
}

#[derive(Clone)]
pub struct FeatureGridProps {
    pub tiles: Vec<FeatureTileProps>,
}
