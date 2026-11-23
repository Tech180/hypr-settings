#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingSize {
    Large,
    Small,
}

/// A second outlined tag after the badge, with its details in a tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderTag {
    pub text: String,
    pub tooltip: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionHeaderProps {
    pub title: String,
    pub subtitle: Option<String>,
    /// Small outlined tag next to the title, such as a connector name.
    pub badge: Option<String>,
    pub tag: Option<HeaderTag>,
    pub size: HeadingSize,
}
