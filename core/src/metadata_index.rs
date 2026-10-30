/// Metadata indexing operations for page backrefs and annotations
///
/// This module provides functions to rebuild page metadata (backrefs.ttl and annotations.ttl)
/// from page content. Used by the `cerbo index` CLI command.
use crate::VaultContext;
use uuid::Uuid;

/// Index all pages across all vaults (two-pass: clear all, rebuild all)
pub fn index_all_pages(global_ctx: &crate::CerboContext) -> Result<IndexStats, String> {
    let mut total_stats = IndexStats::default();

    // Get all vaults
    let vaults = crate::vault::list_all_vaults(global_ctx)?;

    for vault in vaults {
        let vault_ctx = VaultContext::from_path(vault.path.clone())?;
        let stats = index_vault(&vault_ctx)?;
        total_stats.merge(stats);
    }

    Ok(total_stats)
}

/// Rebuild every page's derived metadata in this vault.
///
/// One scan accumulates each target's complete backreference set, then each
/// target is written exactly once. There is no clear-all phase: an interrupted
/// reindex therefore leaves every `backrefs.ttl` holding either its previous or
/// its newly computed content, never an empty file.
pub fn index_vault(vault_ctx: &VaultContext) -> Result<IndexStats, String> {
    use std::collections::{BTreeSet, HashMap};

    let mut stats = IndexStats::default();

    // Phase 1: the live pages, excluding :Ontology objects. Ontologies contain
    // raw RDF/TTL content — backlink and annotation indexing is not meaningful
    // for them, and they are also excluded from cerbo symlink. An object whose
    // meta.ttl cannot be read — two object classes, say — is reported and left
    // untouched rather than indexed as a note.
    let mut page_uuids: Vec<String> = Vec::new();
    for uuid in crate::vault::list_pages_in_vault(&vault_ctx.global, &vault_ctx.vault_path)? {
        match indexable(vault_ctx, &uuid) {
            Ok(true) => page_uuids.push(uuid),
            Ok(false) => {}
            Err(error_message) => stats.errors.push(IndexError { page_uuid: uuid, error_message }),
        }
    }
    let total_pages = page_uuids.len();

    eprintln!("Indexing {} pages...", total_pages);

    // Every live page starts with an empty set, so a page that lost all its
    // inbound links still gets its stale backrefs.ttl replaced.
    let mut inbound: HashMap<&str, BTreeSet<&str>> = page_uuids
        .iter()
        .map(|uuid| (uuid.as_str(), BTreeSet::new()))
        .collect();

    // Phase 2: one scan over the vault.
    for (idx, uuid) in page_uuids.iter().enumerate() {
        let page_md = vault_ctx.object_path(uuid).join("page.md");
        let content = match std::fs::read_to_string(&page_md) {
            Ok(c) => c,
            Err(e) => {
                stats.errors.push(IndexError {
                    page_uuid: uuid.clone(),
                    error_message: format!("Failed to read page.md: {}", e),
                });
                continue;
            }
        };

        for target in crate::links::extract_cerbo_links(&content) {
            stats.links_found += 1;
            match inbound.get_mut(target.as_str()) {
                Some(sources) => {
                    sources.insert(uuid.as_str());
                }
                None => {
                    if !crate::links::is_live_target_vault(vault_ctx, &target) {
                        eprintln!(
                            "Warning: Broken link in page {}: target {} does not exist",
                            uuid, target
                        );
                    }
                }
            }
        }

        let annotations = crate::annotations::extract_annotations(&content);
        stats.annotations_found += annotations.len();
        match crate::annotations::annotations_write_vault(vault_ctx, uuid, &annotations) {
            Ok(warnings) => {
                for warning in warnings {
                    eprintln!("Warning: {}", warning);
                }
            }
            Err(e) => stats.errors.push(IndexError {
                page_uuid: uuid.clone(),
                error_message: format!("Failed to write annotations: {}", e),
            }),
        }

        stats.pages_processed += 1;

        let processed = idx + 1;
        if processed % 10 == 0 || processed == total_pages || processed == 1 {
            eprintln!("Processing {}/{} pages...", processed, total_pages);
        }
    }

    // Phase 3: one write per target, with its complete set of sources.
    for (target, sources) in &inbound {
        let sources: Vec<String> = sources.iter().map(|s| s.to_string()).collect();
        if let Err(e) = crate::links::backrefs_set_vault(vault_ctx, target, &sources) {
            stats.errors.push(IndexError {
                page_uuid: (*target).to_string(),
                error_message: format!("Failed to write backrefs: {}", e),
            });
        }
    }

    Ok(stats)
}

/// Whether this object's links and annotations are indexed: a page that is not
/// an ontology. A page without `meta.ttl` is still a note; one whose `meta.ttl`
/// cannot be read is an error naming the UUID, never a guess.
fn indexable(vault_ctx: &VaultContext, uuid: &str) -> Result<bool, String> {
    let meta_path = vault_ctx.object_path(uuid).join("meta.ttl");
    match crate::object::ObjectMeta::read_from_file(&meta_path) {
        Ok(meta) => Ok(meta.object_type != crate::object::ObjectType::Ontology),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(format!("Failed to read meta.ttl of {}: {}", uuid, e)),
    }
}

/// Index a single page (incremental: update only this page's links and annotations)
pub fn index_page(vault_ctx: &VaultContext, page_uuid: &str) -> Result<IndexStats, String> {
    let mut stats = IndexStats::default();

    if let Err(error_message) = indexable(vault_ctx, page_uuid) {
        stats.errors.push(IndexError {
            page_uuid: page_uuid.to_string(),
            error_message,
        });
        return Ok(stats);
    }

    let page_dir = vault_ctx.object_path(page_uuid);
    let page_md = page_dir.join("page.md");

    // Read page content
    let content = match std::fs::read_to_string(&page_md) {
        Ok(c) => c,
        Err(e) => {
            stats.errors.push(IndexError {
                page_uuid: page_uuid.to_string(),
                error_message: format!("Failed to read page.md: {}", e),
            });
            return Ok(stats);
        }
    };

    // Extract and update links (backrefs)
    let links = crate::links::extract_cerbo_links(&content);
    stats.links_found = links.len();

    // Clear old backrefs for this source page, then re-add
    for target_uuid in &links {
        // Check if target page exists
        let target_path = vault_ctx.object_path(target_uuid);
        if !target_path.exists() {
            eprintln!("Warning: Broken link in page {}: target {} does not exist",
                     page_uuid, target_uuid);
            continue;
        }

        // Remove old backref (if exists)
        let _ = crate::links::backrefs_remove_vault(vault_ctx, target_uuid, page_uuid);
        // Add new backref
        if let Err(e) = crate::links::backrefs_add_vault(vault_ctx, target_uuid, page_uuid) {
            stats.errors.push(IndexError {
                page_uuid: target_uuid.clone(),
                error_message: format!("Failed to add backref: {}", e),
            });
        }
    }

    // Extract and write annotations
    let annotations = crate::annotations::extract_annotations(&content);
    stats.annotations_found = annotations.len();

    match crate::annotations::annotations_write_vault(vault_ctx, page_uuid, &annotations) {
        Ok(warnings) => {
            for warning in warnings {
                eprintln!("Warning: {}", warning);
            }
        }
        Err(e) => stats.errors.push(IndexError {
            page_uuid: page_uuid.to_string(),
            error_message: format!("Failed to write annotations: {}", e),
        }),
    }

    stats.pages_processed = 1;
    Ok(stats)
}

/// Statistics collected during indexing
#[derive(Debug, Default)]
pub struct IndexStats {
    pub pages_processed: usize,
    pub links_found: usize,
    pub annotations_found: usize,
    pub errors: Vec<IndexError>,
}

impl IndexStats {
    fn merge(&mut self, other: IndexStats) {
        self.pages_processed += other.pages_processed;
        self.links_found += other.links_found;
        self.annotations_found += other.annotations_found;
        self.errors.extend(other.errors);
    }
}

/// Error encountered during indexing
#[derive(Debug)]
pub struct IndexError {
    pub page_uuid: String,
    pub error_message: String,
}

/// Outcome of [`backfill_slugs`].
#[derive(Debug, Default)]
pub struct SlugBackfill {
    /// Pages that got a `cerbo:slug`.
    pub updated: usize,
    /// Pages that could not be read or written; each is left as it was.
    pub errors: Vec<IndexError>,
}

/// Backfill `cerbo:slug` into meta.ttl for every page that lacks one.
///
/// One page that fails — a `meta.ttl` with two object classes, say — is
/// reported by UUID and skipped; the rest of the vault is still backfilled.
pub fn backfill_slugs(vault_ctx: &VaultContext) -> Result<SlugBackfill, String> {
    let page_uuids = crate::vault::list_pages_in_vault(&vault_ctx.global, &vault_ctx.vault_path)?;
    let mut report = SlugBackfill::default();

    for uuid in &page_uuids {
        let meta_path = vault_ctx.object_path(uuid).join("meta.ttl");
        if !meta_path.exists() {
            continue;
        }
        let mut meta = match crate::object::ObjectMeta::read_from_file(&meta_path) {
            Ok(meta) => meta,
            Err(e) => {
                report.errors.push(IndexError {
                    page_uuid: uuid.clone(),
                    error_message: format!("Failed to read meta.ttl: {}", e),
                });
                continue;
            }
        };
        if meta.object_type == crate::object::ObjectType::Ontology || meta.slug.is_some() {
            continue;
        }
        let parsed_uuid = Uuid::parse_str(uuid).unwrap_or_else(|_| Uuid::new_v4());
        meta.slug = Some(crate::slug::slugify(&meta.title, parsed_uuid));
        match meta.write_to_file(&meta_path, uuid) {
            Ok(()) => report.updated += 1,
            Err(e) => report.errors.push(IndexError {
                page_uuid: uuid.clone(),
                error_message: format!("Failed to write slug to meta.ttl: {}", e),
            }),
        }
    }

    Ok(report)
}

/// Validate virtual paths across all pages in a vault.
/// Returns a list of `(page_uuid, error_message)` for every page whose
/// `cerbo:virtualPath` value fails validation.
pub fn validate_virtual_paths(vault_ctx: &VaultContext) -> Vec<(String, String)> {
    let Ok(page_uuids) = crate::vault::list_pages_in_vault(&vault_ctx.global, &vault_ctx.vault_path) else {
        return Vec::new();
    };
    let mut errors = Vec::new();

    for uuid in page_uuids {
        let meta_path = vault_ctx.object_path(&uuid).join("meta.ttl");
        if !meta_path.exists() {
            continue;
        }
        let Ok(meta) = crate::object::ObjectMeta::read_from_file(&meta_path) else {
            continue;
        };
        if let Some(vp) = meta.virtual_path
            && let Err(e) = crate::vault::validate_virtual_path(&vp) {
                errors.push((uuid, format!("invalid virtualPath {:?}: {}", vp, e)));
            }
    }

    errors
}

/// Detect path collisions: virtual paths (or slug-derived paths) where two or
/// more pages would materialise at the same symlink location.
/// Returns `Vec<(path, Vec<uuid>)>` for each collision.
pub fn detect_path_collisions(vault_ctx: &VaultContext) -> Vec<(String, Vec<String>)> {
    let Ok(page_uuids) = crate::vault::list_pages_in_vault(&vault_ctx.global, &vault_ctx.vault_path) else {
        return Vec::new();
    };

    let mut path_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for uuid in page_uuids {
        let meta_path = vault_ctx.object_path(&uuid).join("meta.ttl");
        let Ok(meta) = crate::object::ObjectMeta::read_from_file(&meta_path) else {
            continue;
        };

        let effective_path = if let Some(vp) = meta.virtual_path.filter(|s| !s.is_empty()) {
            let leaf = meta.slug.as_deref().unwrap_or(&uuid);
            format!("{}/{}", vp, leaf)
        } else if let Some(s) = meta.slug {
            s
        } else {
            uuid.clone()
        };

        path_map.entry(effective_path).or_default().push(uuid);
    }

    path_map
        .into_iter()
        .filter(|(_, uuids)| uuids.len() > 1)
        .collect()
}

#[cfg(test)]
// Fixtures write files directly; the atomic-write rule guards vault code, not setup.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::{CerboContext, VaultContext};
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_vault() -> (TempDir, VaultContext) {
        let temp = TempDir::new().unwrap();
        let vault_path = temp.path().to_path_buf();

        // Create .cerbo/objects/ structure
        let cerbo_dir = vault_path.join(".cerbo");
        let objects_dir = cerbo_dir.join("objects");
        fs::create_dir_all(&objects_dir).unwrap();

        // Create ontology-map.json
        let map = serde_json::json!({"version": 2, "prefixes": {}});
        fs::write(cerbo_dir.join("ontology-map.json"), serde_json::to_string(&map).unwrap()).unwrap();

        let global = CerboContext {
            config_dir: temp.path().to_path_buf(),
            cache_dir: temp.path().join(".cache"),
        };

        let vault_ctx = VaultContext {
            vault_path: vault_path.clone(),
            global,
        };

        (temp, vault_ctx)
    }

    fn create_test_page(vault_ctx: &VaultContext, uuid: &str, content: &str) {
        let page_dir = vault_ctx.object_path(uuid);
        fs::create_dir_all(&page_dir).unwrap();
        fs::write(page_dir.join("page.md"), content).unwrap();

        // Create minimal meta.ttl
        let meta = format!(r#"@prefix : <cerbo://ontology/> .
<cerbo://objects/{}> :type :Page ; :title "Test" ."#, uuid);
        fs::write(page_dir.join("meta.ttl"), meta).unwrap();
    }

    #[test]
    fn test_index_page_with_links_updates_backrefs() {
        let (_temp, vault_ctx) = setup_test_vault();

        let page1_uuid = "11111111-1111-1111-1111-111111111111";
        let page2_uuid = "22222222-2222-2222-2222-222222222222";

        // Create two pages, page1 links to page2
        create_test_page(&vault_ctx, page1_uuid, &format!("Link to [page2](cerbo://{})", page2_uuid));
        create_test_page(&vault_ctx, page2_uuid, "Page 2 content");

        // Index page1
        let stats = index_page(&vault_ctx, page1_uuid).unwrap();

        assert_eq!(stats.pages_processed, 1);
        assert_eq!(stats.links_found, 1);
        assert_eq!(stats.errors.len(), 0);

        // Check that page2 has a backref to page1
        let backrefs_path = vault_ctx.object_path(page2_uuid).join("backrefs.ttl");
        assert!(backrefs_path.exists(), "backrefs.ttl should be created");

        let backrefs_content = fs::read_to_string(&backrefs_path).unwrap();
        assert!(backrefs_content.contains(page1_uuid), "backrefs should contain source page UUID");
    }

    #[test]
    fn test_index_page_with_annotations_writes_annotations_ttl() {
        let (_temp, vault_ctx) = setup_test_vault();

        let page_uuid = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
        create_test_page(&vault_ctx, page_uuid, "Some [knowledge]{schema:Thing} here");

        // Index page
        let stats = index_page(&vault_ctx, page_uuid).unwrap();

        assert_eq!(stats.pages_processed, 1);
        assert_eq!(stats.annotations_found, 1);
        assert_eq!(stats.errors.len(), 0);

        // Check annotations.ttl was created
        let annotations_path = vault_ctx.object_path(page_uuid).join("annotations.ttl");
        assert!(annotations_path.exists(), "annotations.ttl should be created");

        let annotations_content = fs::read_to_string(&annotations_path).unwrap();
        assert!(annotations_content.contains("knowledge"), "annotations should contain concept text");
        assert!(annotations_content.contains("schema:Thing"), "{annotations_content}");
    }

    #[test]
    fn test_index_page_resolves_prefixes_with_the_vault_map() {
        let (_temp, vault_ctx) = setup_test_vault();
        let map = serde_json::json!({
            "version": 2,
            "prefixes": {"ex": {"namespace": "https://example.org/vocab#"}}
        });
        fs::write(
            vault_ctx.cerbo_dir().join("ontology-map.json"),
            serde_json::to_string(&map).unwrap(),
        )
        .unwrap();

        let page_uuid = "aaaaaaaa-aaaa-aaaa-aaaa-000000000137";
        create_test_page(&vault_ctx, page_uuid, "[Alice]{ex:Person} and [Thing]{zz:Widget}");

        let stats = index_page(&vault_ctx, page_uuid).unwrap();
        assert_eq!(stats.annotations_found, 2);
        assert!(stats.errors.is_empty(), "an unknown prefix is a warning, not an error");

        let content = fs::read_to_string(vault_ctx.object_path(page_uuid).join("annotations.ttl")).unwrap();
        assert!(content.contains("<https://example.org/vocab#Person>"), "{content}");
        assert!(content.contains("cerbo:unresolvedType \"zz:Widget\""), "{content}");
    }

    #[test]
    fn test_index_vault_is_idempotent() {
        let (_temp, vault_ctx) = setup_test_vault();

        let page1_uuid = "11111111-1111-1111-1111-111111111111";
        let page2_uuid = "22222222-2222-2222-2222-222222222222";

        create_test_page(&vault_ctx, page1_uuid, &format!("Link to [page2](cerbo://{})", page2_uuid));
        create_test_page(&vault_ctx, page2_uuid, "Page 2 content");

        // Index vault twice
        let stats1 = index_vault(&vault_ctx).unwrap();
        let stats2 = index_vault(&vault_ctx).unwrap();

        // Both runs should produce same stats
        assert_eq!(stats1.pages_processed, 2);
        assert_eq!(stats2.pages_processed, 2);
        assert_eq!(stats1.links_found, stats2.links_found);
        assert_eq!(stats1.annotations_found, stats2.annotations_found);
        assert_eq!(stats1.errors.len(), 0);
        assert_eq!(stats2.errors.len(), 0);

        // Backref files should be identical
        let backrefs_path = vault_ctx.object_path(page2_uuid).join("backrefs.ttl");
        let backrefs1 = fs::read_to_string(&backrefs_path).unwrap();

        index_vault(&vault_ctx).unwrap();
        let backrefs2 = fs::read_to_string(&backrefs_path).unwrap();

        assert_eq!(backrefs1, backrefs2, "Backrefs should be identical after reindex");
    }

    /// A hub with many inbound links is written once, not once per source.
    #[test]
    fn reindex_writes_each_target_once_with_every_backreference() {
        let (_temp, vault_ctx) = setup_test_vault();

        let hub = "00000000-0000-0000-0000-000000000001";
        create_test_page(&vault_ctx, hub, "# Hub");

        let sources: Vec<String> = (0..137)
            .map(|i| format!("11111111-1111-1111-1111-{:012}", i))
            .collect();
        for uuid in &sources {
            create_test_page(&vault_ctx, uuid, &format!("Link to cerbo://objects/{hub}"));
        }

        let hub_backrefs = vault_ctx.object_path(hub).join("backrefs.ttl");
        let (stats, writes) = crate::fsio::audit::recording(|| index_vault(&vault_ctx).unwrap());

        assert_eq!(stats.errors.len(), 0, "{:?}", stats.errors);
        assert_eq!(
            writes.iter().filter(|p| **p == hub_backrefs).count(),
            1,
            "the hub's backrefs.ttl must be written exactly once per reindex"
        );

        let backrefs = crate::links::backrefs_read_vault(&vault_ctx, hub).unwrap();
        assert_eq!(backrefs.len(), 137, "every source must be recorded");
    }

    /// There is no clear-all phase any more, so an interrupted reindex can never
    /// be observed with an emptied `backrefs.ttl`.
    #[test]
    fn reindex_never_empties_a_backrefs_file_midway() {
        let (_temp, vault_ctx) = setup_test_vault();

        let target = "22222222-2222-2222-2222-222222222222";
        let source = "33333333-3333-3333-3333-333333333333";
        create_test_page(&vault_ctx, target, "# Target");
        create_test_page(&vault_ctx, source, &format!("cerbo://objects/{target}"));

        index_vault(&vault_ctx).unwrap();
        let settled =
            std::fs::read_to_string(vault_ctx.object_path(target).join("backrefs.ttl")).unwrap();
        assert!(settled.contains(source));

        // Every write during a second pass carries the full computed set, so at
        // no point between writes is the file empty.
        let (_, writes) = crate::fsio::audit::recording(|| index_vault(&vault_ctx).unwrap());
        let target_writes = writes
            .iter()
            .filter(|p| p.ends_with("backrefs.ttl") && p.starts_with(vault_ctx.object_path(target)))
            .count();
        assert_eq!(target_writes, 1, "one write means no cleared intermediate state");
        assert_eq!(
            std::fs::read_to_string(vault_ctx.object_path(target).join("backrefs.ttl")).unwrap(),
            settled
        );
    }

    /// A page that lost its last inbound link still gets its stale file replaced.
    #[test]
    fn reindex_clears_backrefs_that_no_longer_apply() {
        let (_temp, vault_ctx) = setup_test_vault();

        let target = "44444444-4444-4444-4444-444444444444";
        let source = "55555555-5555-5555-5555-555555555555";
        create_test_page(&vault_ctx, target, "# Target");
        create_test_page(&vault_ctx, source, &format!("cerbo://objects/{target}"));
        index_vault(&vault_ctx).unwrap();
        assert_eq!(crate::links::backrefs_read_vault(&vault_ctx, target).unwrap().len(), 1);

        create_test_page(&vault_ctx, source, "# Source with no links");
        index_vault(&vault_ctx).unwrap();
        assert!(crate::links::backrefs_read_vault(&vault_ctx, target).unwrap().is_empty());
    }

    /// Trash contents are outside the live object set.
    #[test]
    fn reindex_ignores_the_trash() {
        let (_temp, vault_ctx) = setup_test_vault();

        let live = "88888888-8888-8888-8888-888888888888";
        create_test_page(&vault_ctx, live, "# Live");

        let trashed = vault_ctx
            .vault_path
            .join(".cerbo")
            .join("trash")
            .join("20260904T000000.000000Z-99999999-9999-9999-9999-999999999999");
        std::fs::create_dir_all(&trashed).unwrap();
        std::fs::write(trashed.join("page.md"), "# Trashed [x]{schema:Thing}").unwrap();
        std::fs::write(trashed.join("meta.ttl"), "not even valid turtle").unwrap();

        let stats = index_vault(&vault_ctx).unwrap();

        assert_eq!(stats.pages_processed, 1, "only live pages are indexed");
        assert!(stats.errors.is_empty(), "trash must not be reported: {:?}", stats.errors);
        assert!(!trashed.join("annotations.ttl").exists(), "nothing derived from trash");
        assert!(!trashed.join("backrefs.ttl").exists(), "nothing derived from trash");
    }

    /// Give an indexed page's `meta.ttl` a second object class by hand.
    fn make_ambiguous(vault_ctx: &VaultContext, uuid: &str) {
        let meta_path = vault_ctx.object_path(uuid).join("meta.ttl");
        fs::write(
            &meta_path,
            format!(
                "<cerbo://objects/{uuid}> a <https://schema.org/NoteDigitalDocument> , \
                 <https://schema.org/WebPage> ;\n  <https://schema.org/name> \"Both\" .\n"
            ),
        )
        .unwrap();
    }

    #[test]
    fn an_object_with_two_classes_is_reported_and_not_indexed() {
        let (_temp, vault_ctx) = setup_test_vault();

        let ambiguous = "13713713-0000-4000-8000-000000000137";
        let target = "13713713-0000-4000-8000-000000000001";
        create_test_page(&vault_ctx, target, "# Target");
        create_test_page(
            &vault_ctx,
            ambiguous,
            &format!("[Berlin]{{schema:Place}} cerbo://objects/{target}"),
        );
        make_ambiguous(&vault_ctx, ambiguous);

        let (stats, writes) = crate::fsio::audit::recording(|| index_vault(&vault_ctx).unwrap());

        assert_eq!(stats.pages_processed, 1, "only the readable page");
        assert_eq!(stats.errors.len(), 1, "{:?}", stats.errors);
        assert_eq!(stats.errors[0].page_uuid, ambiguous);
        assert!(stats.errors[0].error_message.contains(ambiguous), "{:?}", stats.errors);
        let dir = vault_ctx.object_path(ambiguous);
        assert!(
            writes.iter().all(|p| !p.starts_with(&dir)),
            "wrote into the ambiguous object: {writes:?}"
        );
        assert!(!dir.join("annotations.ttl").exists());
        // Its links are not counted as backreferences either.
        assert!(crate::links::backrefs_read_vault(&vault_ctx, target).unwrap().is_empty());

        let single = index_page(&vault_ctx, ambiguous).unwrap();
        assert_eq!(single.pages_processed, 0);
        assert_eq!(single.errors.len(), 1);
        assert!(!dir.join("annotations.ttl").exists());
    }

    #[test]
    fn slug_backfill_reports_an_ambiguous_page_and_carries_on() {
        let (_temp, vault_ctx) = setup_test_vault();

        let uuids: Vec<String> = (0..137)
            .map(|n| format!("13700000-0000-4000-8000-{n:012}"))
            .collect();
        for uuid in &uuids {
            create_test_page(&vault_ctx, uuid, "# Page");
        }
        let ambiguous = &uuids[0];
        make_ambiguous(&vault_ctx, ambiguous);
        let ambiguous_before = fs::read(vault_ctx.object_path(ambiguous).join("meta.ttl")).unwrap();

        let report = backfill_slugs(&vault_ctx).unwrap();

        assert_eq!(report.updated, 136);
        assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
        assert_eq!(&report.errors[0].page_uuid, ambiguous);
        assert_eq!(
            fs::read(vault_ctx.object_path(ambiguous).join("meta.ttl")).unwrap(),
            ambiguous_before
        );
        for uuid in uuids.iter().skip(1) {
            let meta = crate::object::ObjectMeta::read_from_file(
                &vault_ctx.object_path(uuid).join("meta.ttl"),
            )
            .unwrap();
            assert!(meta.slug.is_some(), "{uuid} got no slug");
        }
    }

    #[test]
    fn test_index_page_handles_corrupted_file() {
        let (_temp, vault_ctx) = setup_test_vault();

        let page_uuid = "bad-uuid-missing-file";

        // Try to index non-existent page
        let stats = index_page(&vault_ctx, page_uuid).unwrap();

        assert_eq!(stats.pages_processed, 0);
        assert_eq!(stats.errors.len(), 1);
        assert!(stats.errors[0].error_message.contains("Failed to read page.md"));
    }
}
