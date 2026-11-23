use crate::features::display::components::feature_grid::FeatureGridProps;
use crate::features::display::components::hz_group::HzGroupProps;
use crate::shared::settings_group::SettingsGroupProps;

#[derive(Clone)]
pub enum DashboardBlock {
    Group(SettingsGroupProps),
    Hz(HzGroupProps),
    Features(FeatureGridProps),
}

#[derive(Clone)]
pub struct DashboardColumnProps {
    pub blocks: Vec<DashboardBlock>,
}

#[derive(Clone)]
pub struct DisplayDashboardProps {
    pub left: DashboardColumnProps,
    pub right: DashboardColumnProps,
}
