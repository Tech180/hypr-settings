#[path = "interfaces/icon-props.interface.rs"]
mod props;

#[path = "used-icons.rs"]
pub(crate) mod used_icons;
#[path = "icon.component.rs"]
mod component;
#[path = "icon.html.rs"]
mod html;

#[cfg(test)]
mod tests;

pub use component::build;
pub use lucide_static_svg::Icon;
pub use props::IconProps;
