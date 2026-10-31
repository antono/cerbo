use crate::{CerboContext, VaultContext, object};
use regex::Regex;

// ── Link Extraction ─────────────────────────────────────

/// Extract `cerbo://UUID` links from page content (both the bare and the
/// `cerbo://objects/UUID` form).
pub fn extract_cerbo_links(content: &str) -> Vec<String> {
    let re = Regex::new(r"cerbo://(?:objects/)?([a-fA-F0-9-]+)").unwrap();
    re.captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Extract [[Title]] wikilinks from page content (for migration/compatibility)
pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[([^\]]+)\]").unwrap();
    re.captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect()
}

// ── Link Targets ─────────────────────────────────────────

/// True if `target` names a live object.
///
/// The id is parsed as a UUID *before* it is used as a path segment, so a
/// malformed link never reaches the filesystem at all.
pub fn is_live_target(ctx: &CerboContext, target: &str) -> bool {
    uuid::Uuid::parse_str(target).is_ok() && object::object_path(ctx, target).is_dir()
}

/// [`is_live_target`], vault-aware.
pub fn is_live_target_vault(vault_ctx: &VaultContext, target: &str) -> bool {
    uuid::Uuid::parse_str(target).is_ok() && vault_ctx.object_path(target).is_dir()
}

// ── Page Write with Link Extraction ──────────────────────────────

/// Write page content and refresh its links.
///
/// Links live only in the page itself: `object_write` records them in the
/// semantic index and never writes to the objects they point to.
pub fn page_write_with_links(ctx: &CerboContext, uuid: &str, content: &str) -> Result<(), String> {
    object::object_write(ctx, uuid, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object;
    use std::fs;

    /// A vault of its own for each test. The returned `TempDir` guard must stay
    /// alive for the whole test — dropping it removes the directory.
    fn create_test_context() -> (tempfile::TempDir, CerboContext) {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("objects")).unwrap();
        let ctx = CerboContext {
            config_dir: dir.clone(),
            cache_dir: dir.join("cache"),
        };
        (tmp, ctx)
    }

    #[test]
    fn test_extract_cerbo_links() {
        let content = "Links to cerbo://1e5d7dc7-a34c-4a2a-aa49-8bfe2b9805b6 and cerbo://objects/10270714-7005-4627-8b5e-ac75a30c1990 and another cerbo://f7e435db-9740-4a2a-8e57-b439c4f8bb18";
        let links = extract_cerbo_links(content);
        assert_eq!(links.len(), 3);
        assert!(links.contains(&"1e5d7dc7-a34c-4a2a-aa49-8bfe2b9805b6".to_string()));
        assert!(links.contains(&"10270714-7005-4627-8b5e-ac75a30c1990".to_string()));
        assert!(links.contains(&"f7e435db-9740-4a2a-8e57-b439c4f8bb18".to_string()));
    }

    #[test]
    fn test_extract_wikilinks() {
        let content = "Links to [[Page A]] and [[Page B]] and [[Page A]] again";
        let links = extract_wikilinks(content);
        assert_eq!(links.len(), 3);
        assert!(links.contains(&"Page A".to_string()));
        assert!(links.contains(&"Page B".to_string()));
    }

    /// A link to an object that does not exist must not conjure one.
    #[test]
    fn saving_a_link_to_a_missing_object_creates_nothing() {
        let (_tmp, ctx) = create_test_context();
        let source = object::object_create(&ctx, None, object::ObjectType::Product, "Source".into()).unwrap();

        let ghost = "deadbeef-0000-0000-0000-000000000137";
        page_write_with_links(&ctx, &source, &format!("# Source\n\ncerbo://objects/{ghost}"))
            .expect("a broken link must not fail the save");

        assert!(
            !object::object_path(&ctx, ghost).exists(),
            "a link to a missing object fabricated a ghost directory"
        );
        assert_eq!(object::broken_links(&ctx, &format!("cerbo://objects/{ghost}")), vec![ghost.to_string()]);
    }

    /// A malformed id must be rejected before it can be used as a path segment.
    #[test]
    fn malformed_link_id_is_reported_broken_and_never_reaches_the_filesystem() {
        let (_tmp, ctx) = create_test_context();
        let source = object::object_create(&ctx, None, object::ObjectType::Product, "Source".into()).unwrap();

        // Hex-ish enough for the link regex to extract, but not a UUID.
        let malformed = "deadbeef-0000-0000";
        assert!(!is_live_target(&ctx, malformed));

        let objects_before = std::fs::read_dir(object::objects_dir(&ctx)).unwrap().count();
        let content = format!("cerbo://{malformed}");
        page_write_with_links(&ctx, &source, &content).unwrap();

        assert_eq!(object::broken_links(&ctx, &content), vec![malformed.to_string()]);
        assert_eq!(
            std::fs::read_dir(object::objects_dir(&ctx)).unwrap().count(),
            objects_before,
            "a malformed id touched the filesystem"
        );
    }

    /// A save writes only the saved page's own files, never its link targets.
    #[test]
    fn a_save_writes_nothing_outside_the_saved_page() {
        let (_tmp, ctx) = create_test_context();
        let target = object::object_create(&ctx, None, object::ObjectType::Product, "Target".into()).unwrap();
        let source = object::object_create(&ctx, None, object::ObjectType::Product, "Source".into()).unwrap();
        let source_dir = object::object_path(&ctx, &source);

        for content in [format!("# Source\n\ncerbo://objects/{target}"), "# Source\n\nno links".to_string()] {
            let (result, writes) =
                crate::fsio::audit::recording(|| page_write_with_links(&ctx, &source, &content));
            result.unwrap();
            assert!(!writes.is_empty());
            assert!(writes.iter().all(|p| p.starts_with(&source_dir)), "{writes:?}");
        }
    }
}
