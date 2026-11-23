use gtk::prelude::*;
use gtk4 as gtk;

use super::html;
use super::props::{TrialPillProps, TrialPillWidgets};

/// Floating confirmation pill: built once, then shown, hidden and re-worded as the trial ticks.
pub struct TrialPill {
    widgets: TrialPillWidgets,
}

impl TrialPill {
    pub fn new(props: &TrialPillProps) -> Self {
        Self {
            widgets: html::build(props),
        }
    }

    pub fn widget(&self) -> gtk::Widget {
        self.widgets.root.clone().upcast()
    }

    /// `None` hides the pill.
    pub fn set_message(&self, message: Option<&str>) {
        if let Some(message) = message {
            self.widgets.message.set_label(message);
        } else {
            self.set_busy(false);
        }
        self.widgets.root.set_reveal_child(message.is_some());
        // A hidden pill must not swallow clicks meant for the page below it.
        self.widgets.root.set_can_target(message.is_some());
    }

    /// While Keep or Revert is being carried out, a second click would only fail.
    pub fn set_busy(&self, busy: bool) {
        self.widgets.keep.set_sensitive(!busy);
        self.widgets.revert.set_sensitive(!busy);
    }
}
