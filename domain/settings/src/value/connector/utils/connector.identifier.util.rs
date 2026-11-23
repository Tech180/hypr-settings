/// Non-empty `[A-Za-z0-9._-]+`; shared by connector and profile names.
pub fn is_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}
