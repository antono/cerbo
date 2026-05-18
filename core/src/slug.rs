use uuid::Uuid;

/// Derive a URL-safe slug from a title.
/// - Unicode transliteration via the `slug` crate (backed by deunicode)
/// - Forced lowercase
/// - Capped at 80 chars, truncating on a '-' boundary when possible
/// - Falls back to `untitled-<first-8-of-uuid>` when result is empty
pub fn slugify(title: &str, uuid: Uuid) -> String {
    let raw = slug_lib::slugify(title).to_lowercase();

    if raw.is_empty() {
        return format!("untitled-{}", &uuid.to_string()[..8]);
    }

    if raw.len() <= 80 {
        return raw;
    }

    // Truncate at byte boundary 80, prefer cutting at a '-'
    let truncated = &raw[..80];
    if let Some(pos) = truncated.rfind('-') {
        let at_boundary = &truncated[..pos];
        if !at_boundary.is_empty() {
            return at_boundary.to_string();
        }
    }

    truncated.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid() -> Uuid {
        Uuid::parse_str("12345678-1234-1234-1234-123456789012").unwrap()
    }

    #[test]
    fn test_plain_english() {
        assert_eq!(slugify("Rust Ownership", uuid()), "rust-ownership");
    }

    #[test]
    fn test_cyrillic() {
        let s = slugify("Привет мир", uuid());
        assert!(!s.is_empty());
        assert!(s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    }

    #[test]
    fn test_german_umlauts() {
        let s = slugify("Über die Straße", uuid());
        assert!(!s.is_empty());
        assert!(s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    }

    #[test]
    fn test_emoji_transliterates() {
        // Emojis are transliterated by deunicode; result is a valid slug
        let s = slugify("🎉🎊🎈", uuid());
        assert!(!s.is_empty());
        assert!(s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    }

    #[test]
    fn test_empty_after_slugify_falls_back() {
        // Pure punctuation that strips to empty produces untitled-<uuid-prefix>
        let s = slugify("---", uuid());
        assert!(s.starts_with("untitled-"), "got: {s}");
    }

    #[test]
    fn test_length_greater_than_80() {
        let s = slugify(&"a".repeat(100), uuid());
        assert!(s.len() <= 80, "len={}", s.len());
    }

    #[test]
    fn test_length_cap_on_boundary() {
        // "word-" repeated 20 times = 100 chars; should cut on '-' at or before 80
        let title = "word ".repeat(20);
        let s = slugify(&title, uuid());
        assert!(s.len() <= 80, "len={}", s.len());
        assert!(!s.ends_with('-'), "trailing dash: {s}");
    }

    #[test]
    fn test_whitespace_only() {
        let s = slugify("   ", uuid());
        assert!(s.starts_with("untitled-"), "got: {s}");
    }
}
