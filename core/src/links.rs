use crate::{CerboContext, VaultContext, object};
use oxrdf::{Term, Triple};
use regex::Regex;
use std::fs;
use std::path::Path;

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

// ── Backrefs.ttl Management ──────────────────────────────

/// True if `target` names a live object that can hold a backreference.
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

/// Replace an object's entire backreference set in one write.
pub fn backrefs_set_vault(
    vault_ctx: &VaultContext,
    uuid: &str,
    sources: &[String],
) -> Result<(), String> {
    write_backrefs_vault(vault_ctx, uuid, sources)
}

/// Read backrefs.ttl for an object, returns list of UUIDs that link to this object
pub fn backrefs_read(ctx: &CerboContext, uuid: &str) -> Result<Vec<String>, String> {
    let obj_dir = object::object_path(ctx, uuid);
    backrefs_read_from_path(&obj_dir)
}

/// Read backrefs.ttl from a vault context
pub fn backrefs_read_vault(vault_ctx: &VaultContext, uuid: &str) -> Result<Vec<String>, String> {
    let obj_dir = vault_ctx.object_path(uuid);
    backrefs_read_from_path(&obj_dir)
}

fn backrefs_read_from_path(obj_dir: &Path) -> Result<Vec<String>, String> {
    let backrefs_path = obj_dir.join("backrefs.ttl");

    if !backrefs_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&backrefs_path)
        .map_err(|e| format!("Failed to read backrefs.ttl: {}", e))?;

    parse_backrefs(&content)
}

/// Add a backlink to an object's backrefs.ttl (legacy API)
pub fn backrefs_add(ctx: &CerboContext, target_uuid: &str, source_uuid: &str) -> Result<(), String> {
    let mut backrefs = backrefs_read(ctx, target_uuid)?;

    // Avoid duplicates
    if backrefs.contains(&source_uuid.to_string()) {
        return Ok(());
    }

    backrefs.push(source_uuid.to_string());
    write_backrefs(ctx, target_uuid, &backrefs)
}

/// Add a backlink (vault-aware)
pub fn backrefs_add_vault(vault_ctx: &VaultContext, target_uuid: &str, source_uuid: &str) -> Result<(), String> {
    let mut backrefs = backrefs_read_vault(vault_ctx, target_uuid)?;

    if backrefs.contains(&source_uuid.to_string()) {
        return Ok(());
    }

    backrefs.push(source_uuid.to_string());
    write_backrefs_vault(vault_ctx, target_uuid, &backrefs)
}

/// Remove a backlink from an object's backrefs.ttl (legacy API)
pub fn backrefs_remove(ctx: &CerboContext, target_uuid: &str, source_uuid: &str) -> Result<(), String> {
    let mut backrefs = backrefs_read(ctx, target_uuid)?;
    backrefs.retain(|u| u != source_uuid);
    write_backrefs(ctx, target_uuid, &backrefs)
}

/// Remove a backlink (vault-aware)
pub fn backrefs_remove_vault(vault_ctx: &VaultContext, target_uuid: &str, source_uuid: &str) -> Result<(), String> {
    let mut backrefs = backrefs_read_vault(vault_ctx, target_uuid)?;
    backrefs.retain(|u| u != source_uuid);
    write_backrefs_vault(vault_ctx, target_uuid, &backrefs)
}

/// Clear all backlinks for an object (legacy API)
pub fn backrefs_clear(ctx: &CerboContext, uuid: &str) -> Result<(), String> {
    write_backrefs(ctx, uuid, &[])
}

/// Clear all backlinks (vault-aware)
pub fn backrefs_clear_vault(vault_ctx: &VaultContext, uuid: &str) -> Result<(), String> {
    write_backrefs_vault(vault_ctx, uuid, &[])
}

/// Write backrefs.ttl for an object (legacy API)
fn write_backrefs(ctx: &CerboContext, uuid: &str, backrefs: &[String]) -> Result<(), String> {
    let obj_dir = object::object_path(ctx, uuid);
    write_backrefs_to_path(&obj_dir, uuid, backrefs)
}

/// Write backrefs.ttl (vault-aware)
fn write_backrefs_vault(vault_ctx: &VaultContext, uuid: &str, backrefs: &[String]) -> Result<(), String> {
    let obj_dir = vault_ctx.object_path(uuid);
    write_backrefs_to_path(&obj_dir, uuid, backrefs)
}

fn write_backrefs_to_path(obj_dir: &Path, uuid: &str, backrefs: &[String]) -> Result<(), String> {
    // Never conjure the directory: a link to an object that does not exist is a
    // broken link, and creating it here would fabricate a ghost object that
    // outlives the link. Callers detect the broken link via `is_live_target`.
    if !obj_dir.is_dir() {
        return Ok(());
    }

    let subject = crate::rdf::object_iri(uuid);
    let predicate = crate::rdf::cerbo("hasBacklink");
    let triples: Vec<Triple> = backrefs
        .iter()
        .map(|source| {
            Triple::new(
                subject.clone(),
                predicate.clone(),
                Term::from(crate::rdf::object_iri(source)),
            )
        })
        .collect();

    crate::fsio::write_atomic_str(
        &obj_dir.join("backrefs.ttl"),
        &crate::rdf::serialize(&triples),
    )
    .map_err(|e| format!("Failed to write backrefs.ttl: {}", e))
}

/// Parse `backrefs.ttl` and return the UUIDs that link to this object.
fn parse_backrefs(content: &str) -> Result<Vec<String>, String> {
    if let Ok(triples) = crate::rdf::parse(content) {
        let predicate = crate::rdf::cerbo("hasBacklink");
        return Ok(triples
            .iter()
            .filter(|t| t.predicate == predicate)
            .filter_map(|t| match &t.object {
                Term::NamedNode(n) => crate::rdf::uuid_from_object_iri(n.as_str()),
                _ => None,
            })
            .filter(|uuid| *uuid != "none")
            .map(str::to_string)
            .collect());
    }

    // `backrefs.ttl` written before this change is not valid Turtle — its subject
    // was a literal placeholder that is illegal inside an IRI. Scan those instead.
    parse_backrefs_legacy(content)
}

fn parse_backrefs_legacy(content: &str) -> Result<Vec<String>, String> {
    let re = Regex::new(r"cerbo://objects/([a-fA-F0-9-]+)").unwrap();
    let mut backrefs = Vec::new();

    for line in content.lines() {
        if !line.contains(":hasBacklink") {
            continue;
        }
        for cap in re.captures_iter(line) {
            let uuid = cap[1].to_string();
            if uuid != "none" {
                backrefs.push(uuid);
            }
        }
    }

    Ok(backrefs)
}

// ── Page Write with Link Extraction ──────────────────────────────

/// Write page content and refresh its links.
///
/// `object_write` already diffs old against new content and updates every
/// affected object's `backrefs.ttl`; doing it again here doubled the per-save
/// I/O and widened the read-modify-write race for no gain.
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

    #[test]
    fn test_backrefs_add_and_read() {
        let (_tmp, ctx) = create_test_context();

        // Create two objects
        let uuid1 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 1".into()).unwrap();
        let uuid2 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 2".into()).unwrap();

        // Add backlink: uuid2 links to uuid1
        backrefs_add(&ctx, &uuid1, &uuid2).unwrap();

        // Read backrefs for uuid1
        let backrefs = backrefs_read(&ctx, &uuid1).unwrap();
        assert_eq!(backrefs.len(), 1);
        assert!(backrefs.contains(&uuid2));

        // Add another backlink
        let uuid3 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 3".into()).unwrap();
        backrefs_add(&ctx, &uuid1, &uuid3).unwrap();

        let backrefs = backrefs_read(&ctx, &uuid1).unwrap();
        assert_eq!(backrefs.len(), 2);

    }

    #[test]
    fn test_backrefs_remove() {
        let (_tmp, ctx) = create_test_context();

        let uuid1 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 1".into()).unwrap();
        let uuid2 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 2".into()).unwrap();

        // Add then remove
        backrefs_add(&ctx, &uuid1, &uuid2).unwrap();
        let backrefs = backrefs_read(&ctx, &uuid1).unwrap();
        assert_eq!(backrefs.len(), 1);

        backrefs_remove(&ctx, &uuid1, &uuid2).unwrap();
        let backrefs = backrefs_read(&ctx, &uuid1).unwrap();
        assert_eq!(backrefs.len(), 0);

    }

    #[test]
    fn test_page_write_with_links() {
        let (_tmp, ctx) = create_test_context();

        let uuid1 = object::object_create(&ctx, None, object::ObjectType::Product, "Target".into()).unwrap();
        let uuid2 = object::object_create(&ctx, None, object::ObjectType::Product, "Source".into()).unwrap();

        // Write content with link from uuid2 → uuid1
        let content = format!("# Source\n\nLink to cerbo://{}", uuid1);
        page_write_with_links(&ctx, &uuid2, &content).unwrap();

        // Check backrefs on uuid1
        let backrefs = backrefs_read(&ctx, &uuid1).unwrap();
        assert_eq!(backrefs.len(), 1);
        assert!(backrefs.contains(&uuid2));

        // Update content to remove link
        let content = "# Source\n\nNo links anymore.".to_string();
        page_write_with_links(&ctx, &uuid2, &content).unwrap();

        // Check backrefs removed
        let backrefs = backrefs_read(&ctx, &uuid1).unwrap();
        assert_eq!(backrefs.len(), 0);

    }

    #[test]
    fn backrefs_ttl_is_valid_turtle_with_its_own_subject() {
        let (_tmp, ctx) = create_test_context();

        let target = object::object_create(&ctx, None, object::ObjectType::Product, "Target".into()).unwrap();
        let source = object::object_create(&ctx, None, object::ObjectType::Product, "Source".into()).unwrap();
        backrefs_add(&ctx, &target, &source).unwrap();

        let content =
            fs::read_to_string(object::object_path(&ctx, &target).join("backrefs.ttl")).unwrap();

        let triples = crate::rdf::parse(&content).expect("backrefs.ttl must be valid Turtle");
        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].subject.to_string(), format!("<cerbo://objects/{target}>"));
        assert_eq!(triples[0].predicate, crate::rdf::cerbo("hasBacklink"));
        assert_eq!(triples[0].object.to_string(), format!("<cerbo://objects/{source}>"));
    }

    #[test]
    fn object_with_no_backlinks_reads_as_empty() {
        let (_tmp, ctx) = create_test_context();

        let target = object::object_create(&ctx, None, object::ObjectType::Product, "Lonely".into()).unwrap();
        let source = object::object_create(&ctx, None, object::ObjectType::Product, "Source".into()).unwrap();

        backrefs_add(&ctx, &target, &source).unwrap();
        backrefs_remove(&ctx, &target, &source).unwrap();

        let content =
            fs::read_to_string(object::object_path(&ctx, &target).join("backrefs.ttl")).unwrap();
        assert!(crate::rdf::parse(&content).is_ok(), "must stay valid Turtle:\n{content}");
        assert!(backrefs_read(&ctx, &target).unwrap().is_empty());
    }

    /// `backrefs.ttl` from before this change is not valid Turtle; it must still read.
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
        let broken = object::update_backrefs(
            &ctx,
            &source,
            "",
            &format!("cerbo://{malformed}"),
        )
        .unwrap();

        assert_eq!(broken, vec![malformed.to_string()]);
        assert_eq!(
            std::fs::read_dir(object::objects_dir(&ctx)).unwrap().count(),
            objects_before,
            "a malformed id touched the filesystem"
        );
    }

    /// The target used to be diffed twice per save — once in `object_write` and
    /// again here.
    #[test]
    fn one_save_writes_the_targets_backrefs_exactly_once() {
        let (_tmp, ctx) = create_test_context();
        let target = object::object_create(&ctx, None, object::ObjectType::Product, "Target".into()).unwrap();
        let source = object::object_create(&ctx, None, object::ObjectType::Product, "Source".into()).unwrap();
        let backrefs_path = object::object_path(&ctx, &target).join("backrefs.ttl");

        // Adding the link.
        let (result, writes) = crate::fsio::audit::recording(|| {
            page_write_with_links(&ctx, &source, &format!("# Source\n\ncerbo://objects/{target}"))
        });
        result.unwrap();
        assert_eq!(
            writes.iter().filter(|p| **p == backrefs_path).count(),
            1,
            "backrefs.ttl written more than once for one save: {writes:?}"
        );

        // …and removing it again.
        let (result, writes) = crate::fsio::audit::recording(|| {
            page_write_with_links(&ctx, &source, "# Source\n\nno links")
        });
        result.unwrap();
        assert_eq!(
            writes.iter().filter(|p| **p == backrefs_path).count(),
            1,
            "backrefs.ttl written more than once for one save: {writes:?}"
        );
    }

    /// A backreference that cannot be written must fail the save, not vanish.
    #[test]
    #[cfg(unix)]
    fn a_failing_backref_update_fails_the_save() {
        use std::os::unix::fs::PermissionsExt;

        let (_tmp, ctx) = create_test_context();
        let target = object::object_create(&ctx, None, object::ObjectType::Product, "Target".into()).unwrap();
        let source = object::object_create(&ctx, None, object::ObjectType::Product, "Source".into()).unwrap();

        let target_dir = object::object_path(&ctx, &target);
        std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = page_write_with_links(&ctx, &source, &format!("cerbo://objects/{target}"));
        std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o700)).unwrap();

        if result.is_ok() {
            // Running as root defeats the mode bits.
            return;
        }
        let message = result.unwrap_err();
        assert!(message.contains(&target), "the error must name the failed part: {message}");
        assert!(message.contains("backreference"), "{message}");
    }

    #[test]
    fn legacy_backrefs_still_parse() {
        let legacy = format!(
            "@prefix : <cerbo://ontology/> .\n\n<cerbo://objects/<{}>>\n    :hasBacklink <cerbo://objects/aaaaaaaa-0000-0000-0000-000000000137> .\n",
            "uuid"
        );
        assert_eq!(
            parse_backrefs(&legacy).unwrap(),
            vec!["aaaaaaaa-0000-0000-0000-000000000137".to_string()]
        );
    }

    #[test]
    fn test_backrefs_ttl_format() {
        let (_tmp, ctx) = create_test_context();

        let uuid1 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 1".into()).unwrap();
        let uuid2 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 2".into()).unwrap();

        backrefs_add(&ctx, &uuid1, &uuid2).unwrap();

        let obj_dir = object::object_path(&ctx, &uuid1);
        let backrefs_content = fs::read_to_string(obj_dir.join("backrefs.ttl")).unwrap();

        assert!(backrefs_content.contains(":hasBacklink"));
        assert!(backrefs_content.contains(&uuid2));

    }
}
