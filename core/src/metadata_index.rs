/// Metadata indexing operations for page links and annotations
///
/// This module provides functions to rebuild page metadata (annotations.ttl)
/// from page content, report broken links and bring the semantic index up to
/// date. Used by the `cerbo index` CLI command. Backlinks are not stored in the
/// vault; they are a query over the semantic index (`crate::semantic::backlinks`).
use crate::semantic::{IndexOptions, SemanticIndex};
use crate::VaultContext;
use uuid::Uuid;

/// The backlink cache written by Cerbo before the semantic index. Only ever
/// deleted, never read.
const LEGACY_BACKREFS: &str = "backrefs.ttl";

/// Index all pages across all vaults (two-pass: clear all, rebuild all)
pub fn index_all_pages(global_ctx: &crate::CerboContext, options: IndexOptions<'_>) -> Result<IndexStats, String> {
    let mut total_stats = IndexStats::default();

    // Get all vaults
    let vaults = crate::vault::list_all_vaults(global_ctx)?;

    for vault in vaults {
        let vault_ctx = VaultContext::from_path(vault.path.clone())?;
        let stats = index_vault(&vault_ctx, options)?;
        total_stats.merge(stats);
    }

    Ok(total_stats)
}

/// Rebuild every page's derived metadata in this vault, then the vault's
/// semantic index.
///
/// Each page's own `annotations.ttl` is rewritten; link targets are only
/// checked, never written. Leftover `backrefs.ttl` files of live objects are
/// deleted. The semantic index is rebuilt from the files afterwards, so it
/// matches them whatever happened to it before.
pub fn index_vault(vault_ctx: &VaultContext, options: IndexOptions<'_>) -> Result<IndexStats, String> {
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

        stats.links_found += count_links(vault_ctx, uuid, &content);

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

    // Phase 3: files older Cerbo builds wrote into link targets.
    stats.legacy_backrefs_removed = remove_legacy_backrefs(vault_ctx, &mut stats.errors);

    // Phase 4: the graph, from the files as they now are. A missing index is
    // built by `open`; an existing one is rebuilt into a new generation.
    let graph = SemanticIndex::open_existing(vault_ctx, options).and_then(|index| match index {
        Some(mut index) => index.rebuild(options.progress),
        None => SemanticIndex::open(vault_ctx, options).and_then(|index| index.triples()),
    });
    match graph {
        Ok(triples) => stats.triples = triples,
        Err(e) => stats.index_errors.push(e.to_string()),
    }

    Ok(stats)
}

/// Delete the `backrefs.ttl` of every live object, including objects that are
/// not pages; the trash is left alone. Returns how many were deleted.
fn remove_legacy_backrefs(vault_ctx: &VaultContext, errors: &mut Vec<IndexError>) -> usize {
    let Ok(entries) = std::fs::read_dir(vault_ctx.objects_dir()) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if crate::fsio::is_temp_name(&name) {
            continue;
        }
        let path = entry.path().join(LEGACY_BACKREFS);
        match std::fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory) => {}
            Err(e) => errors.push(IndexError {
                page_uuid: name,
                error_message: format!("Failed to delete legacy {}: {}", LEGACY_BACKREFS, e),
            }),
        }
    }
    removed
}

/// Count the `cerbo://` links in a page, warning about each broken one.
fn count_links(vault_ctx: &VaultContext, page_uuid: &str, content: &str) -> usize {
    let links = crate::links::extract_cerbo_links(content);
    for target in &links {
        if !crate::links::is_live_target_vault(vault_ctx, target) {
            eprintln!("Warning: Broken link in page {}: target {} does not exist", page_uuid, target);
        }
    }
    links.len()
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

/// Index a single page (incremental: check this page's links, rewrite its
/// annotations, replace only its triples in the semantic index)
pub fn index_page(vault_ctx: &VaultContext, page_uuid: &str, options: IndexOptions<'_>) -> Result<IndexStats, String> {
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

    // Links are only counted and checked; their targets are never written.
    stats.links_found = count_links(vault_ctx, page_uuid, &content);

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

    // Only this page's graph is replaced; a missing index is built whole.
    let graph = SemanticIndex::open(vault_ctx, options).and_then(|mut index| {
        index.update_objects(&[page_uuid.to_string()])?;
        index.triples()
    });
    match graph {
        Ok(triples) => stats.triples = triples,
        Err(e) => stats.index_errors.push(e.to_string()),
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
    /// Triples in the semantic index once it was updated.
    pub triples: u64,
    /// Leftover `backrefs.ttl` files deleted (full vault runs only).
    pub legacy_backrefs_removed: usize,
    pub errors: Vec<IndexError>,
    /// Why the semantic index could not be updated; the vault files are
    /// written regardless.
    pub index_errors: Vec<String>,
}

impl IndexStats {
    /// Add the counts and errors of `other`, e.g. another vault's run.
    pub fn merge(&mut self, other: IndexStats) {
        self.pages_processed += other.pages_processed;
        self.links_found += other.links_found;
        self.annotations_found += other.annotations_found;
        self.triples += other.triples;
        self.legacy_backrefs_removed += other.legacy_backrefs_removed;
        self.errors.extend(other.errors);
        self.index_errors.extend(other.index_errors);
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
    fn test_index_page_with_links_writes_nothing_into_the_target() {
        let (_temp, vault_ctx) = setup_test_vault();

        let page1_uuid = "11111111-1111-1111-1111-111111111111";
        let page2_uuid = "22222222-2222-2222-2222-222222222222";

        // Create two pages, page1 links to page2
        create_test_page(&vault_ctx, page1_uuid, &format!("Link to [page2](cerbo://{})", page2_uuid));
        create_test_page(&vault_ctx, page2_uuid, "Page 2 content");

        // Index page1
        let (stats, writes) = crate::fsio::audit::recording(|| index_page(&vault_ctx, page1_uuid, IndexOptions::default()).unwrap());

        assert_eq!(stats.pages_processed, 1);
        assert_eq!(stats.links_found, 1);
        assert_eq!(stats.errors.len(), 0);

        let target_dir = vault_ctx.object_path(page2_uuid);
        assert!(writes.iter().all(|p| !p.starts_with(&target_dir)), "{writes:?}");
        assert_eq!(fs::read_dir(&target_dir).unwrap().count(), 2, "only page.md and meta.ttl");
    }

    #[test]
    fn test_index_page_with_annotations_writes_annotations_ttl() {
        let (_temp, vault_ctx) = setup_test_vault();

        let page_uuid = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
        create_test_page(&vault_ctx, page_uuid, "Some [knowledge]{schema:Thing} here");

        // Index page
        let stats = index_page(&vault_ctx, page_uuid, IndexOptions::default()).unwrap();

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

        let stats = index_page(&vault_ctx, page_uuid, IndexOptions::default()).unwrap();
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
        let stats1 = index_vault(&vault_ctx, IndexOptions::default()).unwrap();
        let stats2 = index_vault(&vault_ctx, IndexOptions::default()).unwrap();

        // Both runs should produce same stats
        assert_eq!(stats1.pages_processed, 2);
        assert_eq!(stats2.pages_processed, 2);
        assert_eq!(stats1.links_found, stats2.links_found);
        assert_eq!(stats1.annotations_found, stats2.annotations_found);
        assert_eq!(stats1.errors.len(), 0);
        assert_eq!(stats2.errors.len(), 0);

        // Annotation files should be identical
        let annotations_path = vault_ctx.object_path(page1_uuid).join("annotations.ttl");
        let annotations1 = fs::read_to_string(&annotations_path).unwrap();

        index_vault(&vault_ctx, IndexOptions::default()).unwrap();
        let annotations2 = fs::read_to_string(&annotations_path).unwrap();

        assert_eq!(annotations1, annotations2, "Annotations should be identical after reindex");
    }

    /// A hub with many inbound links gets no file written for them: the only
    /// write into it is its own annotations.ttl.
    #[test]
    fn reindex_writes_nothing_into_link_targets() {
        let (_temp, vault_ctx) = setup_test_vault();

        let hub = "00000000-0000-0000-0000-000000000001";
        create_test_page(&vault_ctx, hub, "# Hub");

        let sources: Vec<String> = (0..137)
            .map(|i| format!("11111111-1111-1111-1111-{:012}", i))
            .collect();
        for uuid in &sources {
            create_test_page(&vault_ctx, uuid, &format!("Link to cerbo://objects/{hub}"));
        }

        let hub_dir = vault_ctx.object_path(hub);
        let (stats, writes) = crate::fsio::audit::recording(|| index_vault(&vault_ctx, IndexOptions::default()).unwrap());

        assert_eq!(stats.errors.len(), 0, "{:?}", stats.errors);
        assert_eq!(stats.links_found, 137);
        let hub_writes: Vec<_> = writes.iter().filter(|p| p.starts_with(&hub_dir)).collect();
        assert_eq!(hub_writes, vec![&hub_dir.join("annotations.ttl")]);
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

        let stats = index_vault(&vault_ctx, IndexOptions::default()).unwrap();

        assert_eq!(stats.pages_processed, 1, "only live pages are indexed");
        assert!(stats.errors.is_empty(), "trash must not be reported: {:?}", stats.errors);
        assert!(!trashed.join("annotations.ttl").exists(), "nothing derived from trash");
    }

    /// Leftover `backrefs.ttl` of every live object goes, the trash keeps its
    /// copy, and the summary counts the files and the index's triples.
    #[test]
    fn full_reindex_deletes_legacy_backrefs_and_counts_triples() {
        let (_temp, vault_ctx) = setup_test_vault();

        let page = "13700000-0000-4000-8000-000000000001";
        let other = "13700000-0000-4000-8000-000000000002";
        create_test_page(&vault_ctx, page, "# Page");
        create_test_page(&vault_ctx, other, &format!("cerbo://{page}"));
        for uuid in [page, other] {
            fs::write(vault_ctx.object_path(uuid).join("backrefs.ttl"), "# legacy").unwrap();
        }
        let trashed = vault_ctx.cerbo_dir().join("trash").join("20260904T000000.000000Z-x");
        fs::create_dir_all(&trashed).unwrap();
        fs::write(trashed.join("backrefs.ttl"), "# legacy").unwrap();

        let stats = index_vault(&vault_ctx, IndexOptions::default()).unwrap();

        assert_eq!(stats.legacy_backrefs_removed, 2);
        assert!(stats.index_errors.is_empty(), "{:?}", stats.index_errors);
        assert!(stats.triples > 0);
        for uuid in [page, other] {
            assert!(!vault_ctx.object_path(uuid).join("backrefs.ttl").exists());
        }
        assert!(trashed.join("backrefs.ttl").exists(), "the trash is not touched");

        let again = index_vault(&vault_ctx, IndexOptions::default()).unwrap();
        assert_eq!(again.legacy_backrefs_removed, 0);
        assert_eq!(again.triples, stats.triples);

        let single = index_page(&vault_ctx, page, IndexOptions::default()).unwrap();
        assert_eq!(single.triples, stats.triples);
        assert_eq!(single.legacy_backrefs_removed, 0);
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

        let (stats, writes) = crate::fsio::audit::recording(|| index_vault(&vault_ctx, IndexOptions::default()).unwrap());

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
        assert_eq!(stats.links_found, 0, "its links are not counted either");

        let single = index_page(&vault_ctx, ambiguous, IndexOptions::default()).unwrap();
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
        let stats = index_page(&vault_ctx, page_uuid, IndexOptions::default()).unwrap();

        assert_eq!(stats.pages_processed, 0);
        assert_eq!(stats.errors.len(), 1);
        assert!(stats.errors[0].error_message.contains("Failed to read page.md"));
    }
}
