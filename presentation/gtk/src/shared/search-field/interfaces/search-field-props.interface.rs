use std::rc::Rc;

#[derive(Clone)]
pub struct SearchFieldProps {
    pub placeholder: String,
    /// Text the field starts with, so a rebuilt page keeps the query.
    pub text: String,
    /// Fires after the user pauses typing, with the whole query.
    pub on_change: Rc<dyn Fn(String)>,
    pub on_activate: Option<Rc<dyn Fn()>>,
    /// Escape pressed with an empty query. Without it, Escape only clears the text.
    pub on_dismiss: Option<Rc<dyn Fn()>>,
}

impl SearchFieldProps {
    pub fn new(placeholder: impl Into<String>, on_change: Rc<dyn Fn(String)>) -> Self {
        Self {
            placeholder: placeholder.into(),
            text: String::new(),
            on_change,
            on_activate: None,
            on_dismiss: None,
        }
    }

    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }
}
