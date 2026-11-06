//! Per-format rules (design D2). Link syntax is shared; what differs is which
//! files are notes, where a title comes from and which frontmatter keys mean
//! something special. A new format is one more [`Dialect`].

use super::Format;
use super::markdown;

/// A frontmatter key that maps to a fixed term instead of `<ns><key>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialKey {
    /// An extra `rdf:type` in the import's namespace.
    Type,
    /// `schema:keywords`, one literal per tag, without a leading `#`.
    Keywords,
    /// `schema:alternateName`; also used to resolve wikilinks.
    AlternateName,
}

pub trait Dialect {
    /// Whether the (non-hidden) file at `rel` is not imported at all. It is
    /// listed in the report as skipped.
    fn skip_file(&self, rel: &str) -> bool;

    /// The page title of a note with file stem `stem` and body `body`.
    fn title(&self, stem: &str, body: &str) -> String;

    /// Whether frontmatter `key` is app state that is dropped.
    fn drop_key(&self, key: &str) -> bool {
        key.starts_with('_')
    }

    fn special_key(&self, key: &str) -> Option<SpecialKey>;
}

pub struct Obsidian;

impl Dialect for Obsidian {
    fn skip_file(&self, rel: &str) -> bool {
        let lower = rel.to_lowercase();
        // Canvas and Excalidraw drawings are plugin data, not notes.
        lower.ends_with(".canvas") || lower.ends_with(".excalidraw.md")
    }

    fn title(&self, stem: &str, _body: &str) -> String {
        stem.to_string()
    }

    fn special_key(&self, key: &str) -> Option<SpecialKey> {
        match key {
            "tags" | "tag" => Some(SpecialKey::Keywords),
            "aliases" | "alias" => Some(SpecialKey::AlternateName),
            _ => None,
        }
    }
}

pub struct Tolaria;

impl Dialect for Tolaria {
    fn skip_file(&self, rel: &str) -> bool {
        // Saved views are app state.
        rel.starts_with("views/")
    }

    fn title(&self, stem: &str, body: &str) -> String {
        markdown::first_h1(body).unwrap_or_else(|| stem.to_string())
    }

    fn special_key(&self, key: &str) -> Option<SpecialKey> {
        (key == "type").then_some(SpecialKey::Type)
    }
}

pub fn dialect_for(format: Format) -> &'static dyn Dialect {
    match format {
        Format::Obsidian => &Obsidian,
        Format::Tolaria => &Tolaria,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolaria_title_is_first_h1() {
        assert_eq!(Tolaria.title("devices", "# Devices\n\nbody"), "Devices");
        assert_eq!(Tolaria.title("devices", "intro\n\n# Devices\n"), "Devices");
    }

    #[test]
    fn tolaria_title_falls_back_to_stem() {
        assert_eq!(Tolaria.title("no-title", "just text\n## Sub\n"), "no-title");
    }

    #[test]
    fn obsidian_title_is_stem_even_with_h1() {
        assert_eq!(Obsidian.title("Rust Ownership", "body"), "Rust Ownership");
        assert_eq!(
            Obsidian.title("Rust Ownership", "# Other\n"),
            "Rust Ownership"
        );
    }

    #[test]
    fn app_state_keys_are_dropped() {
        for dialect in [&Obsidian as &dyn Dialect, &Tolaria] {
            assert!(dialect.drop_key("_favorite"));
            assert!(dialect.drop_key("_favorite_index"));
            assert!(!dialect.drop_key("status"));
        }
    }

    #[test]
    fn special_keys() {
        assert_eq!(Tolaria.special_key("type"), Some(SpecialKey::Type));
        assert_eq!(Tolaria.special_key("tags"), None);
        assert_eq!(Obsidian.special_key("tags"), Some(SpecialKey::Keywords));
        assert_eq!(
            Obsidian.special_key("aliases"),
            Some(SpecialKey::AlternateName)
        );
        assert_eq!(Obsidian.special_key("type"), None);
    }

    #[test]
    fn skip_lists() {
        assert!(Obsidian.skip_file("plan.canvas"));
        assert!(!Obsidian.skip_file("views/x.yml"));
        assert!(Tolaria.skip_file("views/devices.yml"));
        assert!(!Tolaria.skip_file("notes/views.md"));
    }
}
