use crate::shared::badge::Tone;
use crate::shared::icon::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileSize {
    /// 36 px tile that leads a list row.
    #[default]
    Small,
    /// 72 px tile that heads a detail page.
    Large,
}

/// A rounded square holding one glyph; `Tone::Accent` fills and glows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconTileProps {
    pub icon: Icon,
    pub size: TileSize,
    pub tone: Tone,
}

impl IconTileProps {
    pub const fn small(icon: Icon, tone: Tone) -> Self {
        Self {
            icon,
            size: TileSize::Small,
            tone,
        }
    }

    pub const fn large(icon: Icon, tone: Tone) -> Self {
        Self {
            icon,
            size: TileSize::Large,
            tone,
        }
    }
}
