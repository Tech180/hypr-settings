use std::rc::Rc;

use crate::shared::icon::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonKind {
    /// Accent-coloured text, no outline.
    #[default]
    Ghost,
    /// Accent text inside an accent outline.
    Hollow,
    /// Filled with a soft accent tint; the one main action of a page.
    Primary,
    /// Red text inside a red outline, for irreversible actions.
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    /// Fits inside a list row.
    Compact,
    #[default]
    Regular,
    /// Full-width call to action.
    Large,
}

#[derive(Clone)]
pub struct ButtonProps {
    pub label: String,
    pub kind: ButtonKind,
    pub size: ButtonSize,
    pub icon: Option<Icon>,
    pub sensitive: bool,
    pub on_click: Rc<dyn Fn()>,
}

impl ButtonProps {
    pub fn new(label: impl Into<String>, kind: ButtonKind, on_click: Rc<dyn Fn()>) -> Self {
        Self {
            label: label.into(),
            kind,
            size: ButtonSize::default(),
            icon: None,
            sensitive: true,
            on_click,
        }
    }

    #[must_use]
    pub const fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub const fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}
