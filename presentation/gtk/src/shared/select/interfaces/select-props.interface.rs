use std::rc::Rc;

#[derive(Clone)]
pub struct SelectProps {
    pub options: Vec<String>,
    pub selected: usize,
    pub on_select: Rc<dyn Fn(usize)>,
    pub placeholder: Option<String>,
    pub header_label: Option<String>,
    /// Minimum trigger width in pixels (default 160).
    pub min_width: Option<i32>,
}

impl SelectProps {
    pub fn clamped_selected(&self) -> usize {
        if self.options.is_empty() {
            return 0;
        }
        self.selected.min(self.options.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn clamped_selected_stays_in_range() {
        let props = SelectProps {
            options: vec!["a".into(), "b".into(), "c".into()],
            selected: 99,
            on_select: Rc::new(|_| {}),
            placeholder: None,
            header_label: None,
            min_width: None,
        };
        assert_eq!(props.clamped_selected(), 2);
    }
}
