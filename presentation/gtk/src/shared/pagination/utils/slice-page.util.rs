pub fn total_pages(count: usize, per_page: usize) -> usize {
    if per_page == 0 || count == 0 {
        return 0;
    }
    count.div_ceil(per_page)
}

pub fn clamp_page(page: usize, pages: usize) -> usize {
    if pages == 0 {
        return 1;
    }
    page.clamp(1, pages)
}

pub fn slice_page<T>(items: &[T], page: usize, per_page: usize) -> &[T] {
    if per_page == 0 || items.is_empty() {
        return &[];
    }
    let page = page.max(1);
    let start = (page - 1).saturating_mul(per_page);
    if start >= items.len() {
        return &[];
    }
    let end = (start + per_page).min(items.len());
    &items[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_pages_rounds_up() {
        assert_eq!(total_pages(0, 10), 0);
        assert_eq!(total_pages(10, 10), 1);
        assert_eq!(total_pages(11, 10), 2);
    }

    #[test]
    fn slice_returns_window() {
        let items: Vec<i32> = (1..=25).collect();
        assert_eq!(slice_page(&items, 1, 10), &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10][..]);
        assert_eq!(slice_page(&items, 3, 10), &[21, 22, 23, 24, 25][..]);
        assert_eq!(slice_page(&items, 99, 10).len(), 0);
    }

    #[test]
    fn clamp_page_stays_in_range() {
        assert_eq!(clamp_page(5, 3), 3);
        assert_eq!(clamp_page(0, 3), 1);
    }
}
