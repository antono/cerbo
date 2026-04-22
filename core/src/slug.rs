use unicode_normalization::UnicodeNormalization;

/// Derive a filesystem-safe kebab-case slug from a page title.
///
/// Algorithm:
/// 1. NFKD normalise
/// 2. Drop combining diacritics (category Mn)
/// 3. Lowercase
/// 4. Replace runs of non-alphanumeric chars with `-`
/// 5. Trim leading/trailing `-`
pub fn derive_slug(title: &str) -> String {
    // 1 + 2: NFKD, strip diacritics
    let stripped: String = title
        .nfkd()
        .filter(|c| {
            // keep base chars; drop Mn (non-spacing combining marks)
            !unicode_normalization::char::is_combining_mark(*c)
        })
        .collect();

    // 3: lowercase
    let lower = stripped.to_lowercase();

    // 4: non-alphanumeric runs → '-'
    let mut slug = String::with_capacity(lower.len());
    let mut in_sep = false;
    for c in lower.chars() {
        if c.is_alphanumeric() {
            slug.push(c);
            in_sep = false;
        } else if !in_sep {
            slug.push('-');
            in_sep = true;
        }
    }

    // 5: trim leading/trailing '-'
    slug.trim_matches('-').to_string()
}

pub fn slug_from_title(title: String) -> String {
    derive_slug(&title)
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::derive_slug;

    #[test]
    fn ascii_title() {
        assert_eq!(derive_slug("Rust Ownership"), "rust-ownership");
    }

    #[test]
    fn diacritics() {
        assert_eq!(derive_slug("café & résumé"), "cafe-resume");
    }

    #[test]
    fn special_chars() {
        assert_eq!(derive_slug("C++ Basics"), "c-basics");
    }

    #[test]
    fn leading_trailing_punct() {
        assert_eq!(derive_slug("My Tauri App!"), "my-tauri-app");
    }

    #[test]
    fn already_clean() {
        assert_eq!(derive_slug("hello-world"), "hello-world");
    }

    #[test]
    fn empty() {
        assert_eq!(derive_slug(""), "");
    }

    #[test]
    fn only_punct() {
        assert_eq!(derive_slug("---"), "");
    }
}
