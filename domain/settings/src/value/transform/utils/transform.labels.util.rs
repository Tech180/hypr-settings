use crate::value::transform::interfaces::Transform;

impl Transform {
    /// Always explicit: kanshi does not un-rotate when the field is omitted.
    pub fn kanshi_token(self) -> &'static str {
        match self {
            Transform::Normal => "normal",
            Transform::R90 => "90",
            Transform::R180 => "180",
            Transform::R270 => "270",
            Transform::Flipped => "flipped",
            Transform::Flipped90 => "flipped-90",
            Transform::Flipped180 => "flipped-180",
            Transform::Flipped270 => "flipped-270",
        }
    }

    pub fn from_kanshi(token: &str) -> Option<Self> {
        use super::super::constants::ALL;
        ALL.into_iter().find(|t| t.kanshi_token() == token)
    }

    pub fn label(self) -> &'static str {
        match self {
            Transform::Normal => "Landscape",
            Transform::R90 => "Portrait",
            Transform::R180 => "Landscape (flipped)",
            Transform::R270 => "Portrait (flipped)",
            Transform::Flipped => "Mirrored",
            Transform::Flipped90 => "Mirrored 90°",
            Transform::Flipped180 => "Mirrored 180°",
            Transform::Flipped270 => "Mirrored 270°",
        }
    }

    /// Short chip label for the rotation part alone.
    pub fn rotation_label(self) -> &'static str {
        match self.rotation() {
            Transform::Normal => "0°",
            Transform::R90 => "90°",
            Transform::R180 => "180°",
            _ => "270°",
        }
    }
}
