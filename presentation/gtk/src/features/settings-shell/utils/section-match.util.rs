pub fn section_matches(title: &str, keywords: &[&str], query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || title.to_lowercase().contains(&query)
        || keywords.iter().any(|k| k.contains(&query))
}

#[cfg(test)]
mod tests {
    use super::section_matches;

    #[test]
    fn empty_query_matches_everything() {
        assert!(section_matches("Display", &[], "  "));
    }

    #[test]
    fn matches_title_and_keywords_case_insensitively() {
        assert!(section_matches("Display", &["refresh"], "DISP"));
        assert!(section_matches("Display", &["refresh"], "fresh"));
        assert!(!section_matches("Display", &["refresh"], "audio"));
    }
}
