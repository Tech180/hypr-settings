#[path = "interfaces/display-dashboard-props.interface.rs"]
mod props;

#[path = "display-dashboard.component.rs"]
mod component;
#[path = "display-dashboard.html.rs"]
mod html;

pub use component::build;
pub use props::{DashboardBlock, DashboardColumnProps, DisplayDashboardProps};
