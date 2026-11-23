use lucide_static_svg::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconProps {
    pub icon: Icon,
    /// Edge length in pixels.
    pub size: i32,
}
