use std::rc::Rc;

#[derive(Clone)]
pub struct PaginationProps {
    /// Current page (1-based).
    pub page: usize,
    pub total_pages: usize,
    pub on_page: Rc<dyn Fn(usize)>,
    pub page_size: usize,
    pub page_size_options: Vec<usize>,
    pub on_page_size: Rc<dyn Fn(usize)>,
    /// Distinguishes multiple pagers on one screen (e.g. Saved vs Available Wi‑Fi).
    pub accessible_label: Option<String>,
}
