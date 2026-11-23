#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageItem {
    Number(u32),
    Ellipsis,
}

/// Page numbers and ellipses for the control strip (`current` and `total` are 1-based).
pub fn page_window(current: u32, total: u32) -> Vec<PageItem> {
    if total == 0 {
        return Vec::new();
    }
    if total <= 7 {
        return (1..=total).map(PageItem::Number).collect();
    }
    let current = current.clamp(1, total);
    let mut items = Vec::new();
    items.push(PageItem::Number(1));

    let window_start = current.saturating_sub(1).max(2);
    let window_end = (current + 1).min(total - 1);

    if window_start > 2 {
        items.push(PageItem::Ellipsis);
    }
    for page in window_start..=window_end {
        items.push(PageItem::Number(page));
    }
    if window_end < total - 1 {
        items.push(PageItem::Ellipsis);
    }
    items.push(PageItem::Number(total));
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_total_lists_every_page() {
        assert_eq!(
            page_window(2, 5),
            vec![
                PageItem::Number(1),
                PageItem::Number(2),
                PageItem::Number(3),
                PageItem::Number(4),
                PageItem::Number(5),
            ]
        );
    }

    #[test]
    fn large_total_uses_ellipses() {
        let window = page_window(5, 10);
        assert!(window.contains(&PageItem::Ellipsis));
        assert_eq!(window.first(), Some(&PageItem::Number(1)));
        assert_eq!(window.last(), Some(&PageItem::Number(10)));
        assert!(window.contains(&PageItem::Number(5)));
    }

    #[test]
    fn empty_total_is_empty() {
        assert!(page_window(1, 0).is_empty());
    }
}
