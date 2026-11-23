use gtk4 as gtk;

/// What the shell needs from a settings section, so new sections plug in without shell changes.
pub trait Section {
    fn widget(&self) -> gtk::Widget;
    fn refresh(&self);
    /// Called when the window is closing; must call `proceed` exactly once when it is safe.
    fn on_close(&self, proceed: Box<dyn FnOnce()>);
}
