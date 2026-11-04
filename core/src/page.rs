use serde::Serialize;
use crate::CerboContext;
use crate::object::{object_path, ObjectMeta};

#[derive(Debug, Serialize)]
pub struct PageMeta {
    pub uuid: String,
    pub title: String,
    /// The page's `cerbo:virtualPath`; `None` at the root or when the stored
    /// value is invalid (spec `page-crud`, "List pages").
    #[serde(rename = "virtualPath")]
    pub virtual_path: Option<String>,
}

/// Create a new page using UUID model.
/// Returns the generated UUID.
pub fn page_create(ctx: &CerboContext, title: String) -> Result<String, String> {
    crate::object::object_create(ctx, None, crate::object::ObjectType::Product, title)
}

/// Create a new page with custom metadata (slug and virtual path).
/// Returns the generated UUID.
pub fn page_create_with_metadata(
    ctx: &CerboContext,
    title: String,
    slug: Option<String>,
    virtual_path: Option<String>,
) -> Result<String, String> {
    crate::object::object_create_with_metadata(ctx, None, crate::object::ObjectType::Product, title, slug, virtual_path)
}

/// Read page content by UUID.
pub fn page_read(ctx: &CerboContext, uuid: String) -> Result<String, String> {
    crate::object::object_read(ctx, &uuid)
}

/// Write page content by UUID. Fails if type is Source (read-only).
/// Returns the content on success.
///
/// Every save — from the CLI or the desktop editor — refreshes the page's
/// `annotations.ttl` and its graph in the semantic index, and writes no other
/// object's files.
pub fn page_write(
    ctx: &CerboContext,
    uuid: String,
    content: String,
) -> Result<String, String> {
    crate::annotations::page_write_with_annotations(ctx, &uuid, &content)?;
    Ok(content)
}

/// Delete a page by UUID. Fails if type is Source (read-only).
pub fn page_delete(ctx: &CerboContext, uuid: String) -> Result<(), String> {
    crate::object::object_delete(ctx, &uuid)
}

/// Update the title of a page: writes new title to meta.ttl and replaces the first H1 in page.md.
pub fn page_update_title(ctx: &CerboContext, uuid: String, new_title: String) -> Result<(), String> {
    let obj_dir = object_path(ctx, &uuid);
    let meta_path = obj_dir.join("meta.ttl");
    let page_path = obj_dir.join("page.md");

    // Update meta.ttl
    let mut meta = ObjectMeta::read_from_file(&meta_path)
        .map_err(|e| format!("page_update_title: read meta.ttl: {e}"))?;
    meta.title = new_title.clone();
    meta.write_to_file(&meta_path, &uuid)
        .map_err(|e| format!("page_update_title: write meta.ttl: {e}"))?;

    // Update first H1 in page.md
    if page_path.exists() {
        let content = std::fs::read_to_string(&page_path)
            .map_err(|e| format!("page_update_title: read page.md: {e}"))?;
        let updated = if let Some(pos) = content.find('\n') {
            let first_line = &content[..pos];
            if first_line.trim().starts_with("# ") {
                format!("# {}{}", new_title, &content[pos..])
            } else {
                content
            }
        } else if content.trim().starts_with("# ") {
            format!("# {}", new_title)
        } else {
            content
        };
        crate::fsio::write_atomic_str(&page_path, &updated)
            .map_err(|e| format!("page_update_title: write page.md: {e}"))?;
    }

    Ok(())
}

/// Set or clear a page's `cerbo:virtualPath` (spec `page-virtual-path`, "Move
/// page to a virtual path"). `None` or an empty path moves the page to the
/// root. The path is validated before anything is read; only the page's own
/// `meta.ttl` is written, and its graph in the semantic index is refreshed.
/// Source pages may be moved: placement is metadata, not content.
pub fn page_update_virtual_path(
    ctx: &CerboContext,
    uuid: String,
    path: Option<String>,
) -> Result<(), String> {
    let path = normalize_virtual_path(path)?;
    if set_virtual_path(ctx, &uuid, path)? {
        crate::object::sync_index(ctx, &uuid);
    }
    Ok(())
}

/// Outcome of [`folder_rename`]: the pages whose path changed, and the pages
/// under the prefix that could not be rewritten, each with its error.
#[derive(Debug, Default, Serialize)]
pub struct FolderRenameReport {
    pub changed: Vec<String>,
    pub failed: Vec<(String, String)>,
}

/// Replace the folder prefix `old_prefix` with `new_prefix` on every page whose
/// `cerbo:virtualPath` is `old_prefix` or starts with `old_prefix/` (spec
/// `page-virtual-path`, "Rename a folder prefix"). Both prefixes must be
/// non-empty and valid; that is checked before any page is written. Each page
/// gets its own atomic write; a page that fails is reported and the rest are
/// still rewritten. The semantic index is refreshed once for all changed pages.
pub fn folder_rename(
    ctx: &CerboContext,
    old_prefix: &str,
    new_prefix: &str,
) -> Result<FolderRenameReport, String> {
    for (what, prefix) in [("old", old_prefix), ("new", new_prefix)] {
        if prefix.is_empty() {
            return Err(format!("{what} folder path must not be empty"));
        }
        crate::vault::validate_virtual_path(prefix)
            .map_err(|e| format!("invalid {what} folder path {prefix:?}: {e}"))?;
    }

    let nested = format!("{old_prefix}/");
    let mut report = FolderRenameReport::default();
    for page in page_list(ctx)? {
        let Some(current) = page.virtual_path else { continue };
        let rest = if current == old_prefix {
            ""
        } else if let Some(rest) = current.strip_prefix(&nested) {
            rest
        } else {
            continue;
        };
        let target = if rest.is_empty() {
            new_prefix.to_string()
        } else {
            format!("{new_prefix}/{rest}")
        };
        match set_virtual_path(ctx, &page.uuid, Some(target)) {
            Ok(true) => report.changed.push(page.uuid),
            Ok(false) => {} // already there (old == new, or a merge of equals)
            Err(e) => report.failed.push((page.uuid, e)),
        }
    }

    if !report.changed.is_empty()
        && let Some(vault_ctx) = crate::object::vault_of(ctx)
    {
        crate::semantic::sync_objects(&vault_ctx, &report.changed);
    }
    Ok(report)
}

/// An empty path means the root; anything else must validate.
fn normalize_virtual_path(path: Option<String>) -> Result<Option<String>, String> {
    let path = path.filter(|p| !p.is_empty());
    if let Some(p) = &path {
        crate::vault::validate_virtual_path(p)
            .map_err(|e| format!("invalid virtualPath {p:?}: {e}"))?;
    }
    Ok(path)
}

/// Write `path` into the page's `meta.ttl` without touching the index.
/// Returns whether the file was written (`false` when the path already matched).
fn set_virtual_path(ctx: &CerboContext, uuid: &str, path: Option<String>) -> Result<bool, String> {
    let meta_path = object_path(ctx, uuid).join("meta.ttl");
    if !meta_path.exists() {
        return Err(format!("Object not found: {uuid}"));
    }
    let mut meta = ObjectMeta::read_from_file(&meta_path)
        .map_err(|e| format!("page_update_virtual_path: read meta.ttl: {e}"))?;
    if meta.virtual_path.as_deref().filter(|p| !p.is_empty()) == path.as_deref() {
        // The reader drops an invalid stored value, which then reads as the
        // root; a move to the root must still erase it from disk.
        let stale_on_disk = path.is_none()
            && std::fs::read_to_string(&meta_path)
                .map(|raw| raw.contains("virtualPath"))
                .unwrap_or(false);
        if !stale_on_disk {
            return Ok(false);
        }
    }
    meta.virtual_path = path;
    meta.write_to_file(&meta_path, uuid)
        .map_err(|e| format!("page_update_virtual_path: write meta.ttl: {e}"))?;
    Ok(true)
}

/// List all pages by scanning .cerbo/objects/ for directories with page.md.
pub fn page_list(ctx: &CerboContext) -> Result<Vec<PageMeta>, String> {
    let objects_dir = crate::object::objects_dir(ctx);
    let mut pages = Vec::new();

    if !objects_dir.exists() {
        return Ok(pages);
    }

    let entries = std::fs::read_dir(&objects_dir)
        .map_err(|e| format!("page_list read_dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("page_list entry: {}", e))?;
        let path = entry.path();
        let uuid = entry.file_name().to_string_lossy().to_string();
        // Leftovers from an interrupted atomic write are not objects.
        if crate::fsio::is_temp_name(&uuid) {
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        let page_md = path.join("page.md");
        if !page_md.exists() {
            continue;
        }

        // `meta.ttl` is the title's only home; the H1 in `page.md` is a copy the
        // user may have edited outside Cerbo, and is never consulted here.
        let (title, virtual_path) = match ObjectMeta::read_from_file(&path.join("meta.ttl")) {
            Ok(meta) => (Some(meta.title), meta.virtual_path.filter(|p| !p.is_empty())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (None, None),
            // An object with two object classes is neither a page nor anything
            // else until the user fixes it; say so rather than list it.
            Err(e) => {
                eprintln!("Warning: skipping object {uuid}: {e}");
                continue;
            }
        };
        let title = title
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "Untitled".to_string());

        pages.push(PageMeta { uuid, title, virtual_path });
    }

    pages.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(pages)
}

#[cfg(test)]
// Fixtures write files directly; the atomic-write rule guards vault code, not setup.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::object::{object_create, ObjectType};

    fn test_context() -> (tempfile::TempDir, CerboContext) {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        std::fs::create_dir_all(dir.join("objects")).unwrap();
        let ctx = CerboContext {
            config_dir: dir.clone(),
            cache_dir: dir.join("cache"),
        };
        (tmp, ctx)
    }

    #[test]
    fn page_list_empty_vault() {
        let (_tmp, ctx) = test_context();
        assert!(page_list(&ctx).unwrap().is_empty());
    }

    #[test]
    fn page_list_takes_the_title_from_meta_not_the_body() {
        let (_tmp, ctx) = test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Recorded Title".into()).unwrap();

        // Someone edited the H1 outside Cerbo.
        let page_md = object_path(&ctx, &uuid).join("page.md");
        std::fs::write(&page_md, "# Edited Elsewhere\n\nbody\n").unwrap();

        let pages = page_list(&ctx).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Recorded Title");
    }

    #[test]
    fn page_list_does_not_read_page_md() {
        let (_tmp, ctx) = test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Only In Meta".into()).unwrap();

        // An unreadable body must not stop the page from being listed.
        let page_md = object_path(&ctx, &uuid).join("page.md");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&page_md, std::fs::Permissions::from_mode(0o000)).unwrap();
        }

        let pages = page_list(&ctx).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&page_md, std::fs::Permissions::from_mode(0o644)).unwrap();
        }

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Only In Meta");
    }

    #[test]
    fn renaming_to_a_quoted_title_keeps_meta_valid() {
        let (_tmp, ctx) = test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Before".into()).unwrap();

        let new_title = r#"He said "hi""#.to_string();
        page_update_title(&ctx, uuid.clone(), new_title.clone()).unwrap();

        let meta_path = object_path(&ctx, &uuid).join("meta.ttl");
        let content = std::fs::read_to_string(&meta_path).unwrap();
        assert!(crate::rdf::parse(&content).is_ok(), "meta.ttl must stay valid Turtle:\n{content}");

        assert_eq!(ObjectMeta::read_from_file(&meta_path).unwrap().title, new_title);
        assert_eq!(page_list(&ctx).unwrap()[0].title, new_title);
    }

    fn annotations_of(ctx: &CerboContext, uuid: &str) -> String {
        std::fs::read_to_string(object_path(ctx, uuid).join("annotations.ttl")).unwrap()
    }

    #[test]
    fn saving_a_page_writes_its_annotations() {
        let (_tmp, ctx) = test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Trip".into()).unwrap();

        page_write(&ctx, uuid.clone(), "I visited [Berlin]{schema:Place}.\n".into()).unwrap();
        let doc = annotations_of(&ctx, &uuid);
        assert!(doc.contains("schema:mentions") && doc.contains("\"Berlin\""), "{doc}");
        assert!(doc.contains("schema:Place") && doc.contains("\"1:10\""), "{doc}");

        // An annotation added later shows up without an index run.
        page_write(
            &ctx,
            uuid.clone(),
            "I visited [Berlin]{schema:Place} with [Alice]{foaf:Person}.\n".into(),
        )
        .unwrap();
        let doc = annotations_of(&ctx, &uuid);
        assert!(doc.contains("<http://xmlns.com/foaf/0.1/Person>"), "{doc}");
    }

    #[test]
    fn saving_a_page_without_annotations_leaves_no_mentions() {
        let (_tmp, ctx) = test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Trip".into()).unwrap();
        page_write(&ctx, uuid.clone(), "I visited [Berlin]{schema:Place}.\n".into()).unwrap();

        page_write(&ctx, uuid.clone(), "I visited Berlin.\n".into()).unwrap();

        let doc = annotations_of(&ctx, &uuid);
        let triples = crate::rdf::parse(&doc).unwrap();
        assert!(
            triples.iter().all(|t| t.predicate != crate::rdf::schema("mentions")),
            "{doc}"
        );
        assert!(!doc.contains("Berlin"), "{doc}");
    }

    #[test]
    fn saving_an_ontology_writes_no_annotations() {
        let (_tmp, ctx) = test_context();
        let uuid = object_create(&ctx, None, ObjectType::Ontology, "Vocab".into()).unwrap();

        page_write(&ctx, uuid.clone(), "[Thing]{schema:Thing}\n".into()).unwrap();

        assert!(!object_path(&ctx, &uuid).join("annotations.ttl").exists());
    }

    #[test]
    fn page_list_skips_an_object_with_two_classes() {
        let (_tmp, ctx) = test_context();
        let good = object_create(&ctx, None, ObjectType::Product, "Fine".into()).unwrap();
        let bad = object_create(&ctx, None, ObjectType::Product, "Both".into()).unwrap();
        let meta_path = object_path(&ctx, &bad).join("meta.ttl");
        let mut content = std::fs::read_to_string(&meta_path).unwrap();
        content.push_str(&format!("<cerbo://objects/{bad}> a <https://schema.org/WebPage> .\n"));
        std::fs::write(&meta_path, content).unwrap();

        let pages = page_list(&ctx).unwrap();
        assert_eq!(pages.len(), 1, "{pages:?}");
        assert_eq!(pages[0].uuid, good);
    }

    #[test]
    fn page_list_skips_leftover_temp_files() {
        let (_tmp, ctx) = test_context();
        let uuid = object_create(&ctx, None, ObjectType::Product, "Real Page".into()).unwrap();

        // A crash between write and rename can leave either of these behind.
        let objects = crate::object::objects_dir(&ctx);
        std::fs::write(objects.join(format!("{}stray", crate::fsio::TEMP_PREFIX)), "junk").unwrap();
        let stray_dir = objects.join(format!("{}stray-dir", crate::fsio::TEMP_PREFIX));
        std::fs::create_dir_all(&stray_dir).unwrap();
        std::fs::write(stray_dir.join("page.md"), "# Ghost\n").unwrap();

        let pages = page_list(&ctx).unwrap();
        assert_eq!(pages.len(), 1, "temp leftovers must not be listed: {pages:?}");
        assert_eq!(pages[0].uuid, uuid);
    }

    // ── virtual path (spec `page-virtual-path`) ──────────────────────────────

    use crate::object::object_create_with_metadata;

    fn create_at(ctx: &CerboContext, title: &str, path: Option<&str>) -> String {
        object_create_with_metadata(
            ctx,
            None,
            ObjectType::Product,
            title.into(),
            None,
            path.map(str::to_string),
        )
        .unwrap()
    }

    fn meta_of(ctx: &CerboContext, uuid: &str) -> ObjectMeta {
        ObjectMeta::read_from_file(&object_path(ctx, uuid).join("meta.ttl")).unwrap()
    }

    fn raw_meta(ctx: &CerboContext, uuid: &str) -> String {
        std::fs::read_to_string(object_path(ctx, uuid).join("meta.ttl")).unwrap()
    }

    fn path_of(ctx: &CerboContext, uuid: &str) -> Option<String> {
        page_list(ctx)
            .unwrap()
            .into_iter()
            .find(|p| p.uuid == uuid)
            .unwrap()
            .virtual_path
    }

    #[test]
    fn page_list_reports_virtual_path_or_none() {
        let (_tmp, ctx) = test_context();
        let nested = create_at(&ctx, "Ownership", Some("notes/rust"));
        let root = create_at(&ctx, "Home", None);

        assert_eq!(path_of(&ctx, &nested).as_deref(), Some("notes/rust"));
        assert_eq!(path_of(&ctx, &root), None);
    }

    #[test]
    fn move_page_into_folder_writes_only_meta() {
        let (_tmp, ctx) = test_context();
        let uuid = create_at(&ctx, "Home", None);
        let before = meta_of(&ctx, &uuid);
        let page_before = std::fs::read_to_string(object_path(&ctx, &uuid).join("page.md")).unwrap();

        page_update_virtual_path(&ctx, uuid.clone(), Some("notes/rust".into())).unwrap();

        let after = meta_of(&ctx, &uuid);
        assert_eq!(after.virtual_path.as_deref(), Some("notes/rust"));
        assert_eq!(after.slug, before.slug);
        assert_eq!(after.title, before.title);
        assert_eq!(after.created, before.created);
        let page_after = std::fs::read_to_string(object_path(&ctx, &uuid).join("page.md")).unwrap();
        assert_eq!(page_after, page_before);
    }

    #[test]
    fn move_page_to_root_removes_the_predicate() {
        let (_tmp, ctx) = test_context();
        let uuid = create_at(&ctx, "Ownership", Some("notes/rust"));
        assert!(raw_meta(&ctx, &uuid).contains("virtualPath"));

        page_update_virtual_path(&ctx, uuid.clone(), Some(String::new())).unwrap();
        assert!(!raw_meta(&ctx, &uuid).contains("virtualPath"));
        assert_eq!(path_of(&ctx, &uuid), None);

        // `None` means the same as the empty string.
        page_update_virtual_path(&ctx, uuid.clone(), Some("a".into())).unwrap();
        page_update_virtual_path(&ctx, uuid.clone(), None).unwrap();
        assert!(!raw_meta(&ctx, &uuid).contains("virtualPath"));
    }

    #[test]
    fn move_page_to_its_current_path_is_a_noop() {
        let (_tmp, ctx) = test_context();
        let uuid = create_at(&ctx, "Index", Some("notes"));
        let before = raw_meta(&ctx, &uuid);
        let mtime = std::fs::metadata(object_path(&ctx, &uuid).join("meta.ttl")).unwrap().modified().unwrap();

        page_update_virtual_path(&ctx, uuid.clone(), Some("notes".into())).unwrap();

        assert_eq!(raw_meta(&ctx, &uuid), before);
        let after = std::fs::metadata(object_path(&ctx, &uuid).join("meta.ttl")).unwrap().modified().unwrap();
        assert_eq!(after, mtime, "meta.ttl was rewritten for a no-op move");
    }

    #[test]
    fn move_to_root_erases_an_invalid_stored_path() {
        let (_tmp, ctx) = test_context();
        let uuid = create_at(&ctx, "Odd", Some("notes"));
        let meta_path = object_path(&ctx, &uuid).join("meta.ttl");
        let raw = std::fs::read_to_string(&meta_path).unwrap().replace("\"notes\"", "\"/bad/\"");
        std::fs::write(&meta_path, raw).unwrap();
        assert!(raw_meta(&ctx, &uuid).contains("virtualPath"));
        assert_eq!(path_of(&ctx, &uuid), None, "an invalid value reads as the root");

        page_update_virtual_path(&ctx, uuid.clone(), None).unwrap();
        assert!(!raw_meta(&ctx, &uuid).contains("virtualPath"));

        // A root page stays untouched by a second move to the root.
        let mtime = std::fs::metadata(&meta_path).unwrap().modified().unwrap();
        page_update_virtual_path(&ctx, uuid.clone(), None).unwrap();
        assert_eq!(std::fs::metadata(&meta_path).unwrap().modified().unwrap(), mtime);
    }

    #[test]
    fn folder_rename_onto_itself_changes_nothing() {
        let (_tmp, ctx) = test_context();
        create_at(&ctx, "A", Some("notes"));
        create_at(&ctx, "B", Some("notes/rust"));
        let report = folder_rename(&ctx, "notes", "notes").unwrap();
        assert!(report.changed.is_empty());
        assert!(report.failed.is_empty());
    }

    #[test]
    fn move_page_rejects_invalid_paths_without_writing() {
        let (_tmp, ctx) = test_context();
        let uuid = create_at(&ctx, "Index", Some("notes"));
        let before = raw_meta(&ctx, &uuid);

        for (bad, rule) in [
            ("/notes", "leading or trailing slash"),
            ("notes/", "leading or trailing slash"),
            ("notes//rust", "empty segment"),
            ("notes/../x", "dot or dotdot segment"),
            ("notes/./x", "dot or dotdot segment"),
            ("no\0tes", "NUL byte"),
        ] {
            let err = page_update_virtual_path(&ctx, uuid.clone(), Some(bad.into())).unwrap_err();
            assert!(err.contains(rule), "{bad:?}: {err}");
            assert_eq!(raw_meta(&ctx, &uuid), before, "{bad:?} wrote meta.ttl");
        }
    }

    #[test]
    fn move_unknown_page_fails() {
        let (_tmp, ctx) = test_context();
        let err = page_update_virtual_path(&ctx, "00000000-0000-0000-0000-000000000000".into(), Some("notes".into()))
            .unwrap_err();
        assert!(err.contains("Object not found"), "{err}");
    }

    #[test]
    fn move_source_page_is_allowed() {
        let (_tmp, ctx) = test_context();
        let uuid = object_create(&ctx, None, ObjectType::Source, "Imported".into()).unwrap();
        let page_before = std::fs::read_to_string(object_path(&ctx, &uuid).join("page.md")).unwrap();

        page_update_virtual_path(&ctx, uuid.clone(), Some("imports".into())).unwrap();

        assert_eq!(meta_of(&ctx, &uuid).virtual_path.as_deref(), Some("imports"));
        assert_eq!(meta_of(&ctx, &uuid).object_type, ObjectType::Source);
        assert_eq!(std::fs::read_to_string(object_path(&ctx, &uuid).join("page.md")).unwrap(), page_before);
    }

    #[test]
    fn move_page_writes_no_other_object() {
        let (_tmp, ctx) = test_context();
        let others: Vec<String> = (0..137).map(|i| create_at(&ctx, &format!("Page {i}"), Some("bulk"))).collect();
        let snapshot: Vec<std::time::SystemTime> = others
            .iter()
            .map(|u| std::fs::metadata(object_path(&ctx, u).join("meta.ttl")).unwrap().modified().unwrap())
            .collect();
        let moved = create_at(&ctx, "Mover", None);

        page_update_virtual_path(&ctx, moved, Some("bulk".into())).unwrap();

        for (u, before) in others.iter().zip(snapshot) {
            let after = std::fs::metadata(object_path(&ctx, u).join("meta.ttl")).unwrap().modified().unwrap();
            assert_eq!(after, before, "{u} was rewritten");
        }
    }

    #[test]
    fn folder_rename_moves_nested_pages_and_leaves_lookalikes() {
        let (_tmp, ctx) = test_context();
        let a = create_at(&ctx, "A", Some("notes"));
        let b = create_at(&ctx, "B", Some("notes/rust"));
        let c = create_at(&ctx, "C", Some("notes/rust/async"));
        let d = create_at(&ctx, "D", Some("notebook"));
        let e = create_at(&ctx, "E", None);

        let report = folder_rename(&ctx, "notes", "archive/notes").unwrap();

        assert_eq!(report.changed.len(), 3);
        assert!(report.failed.is_empty());
        assert_eq!(path_of(&ctx, &a).as_deref(), Some("archive/notes"));
        assert_eq!(path_of(&ctx, &b).as_deref(), Some("archive/notes/rust"));
        assert_eq!(path_of(&ctx, &c).as_deref(), Some("archive/notes/rust/async"));
        assert_eq!(path_of(&ctx, &d).as_deref(), Some("notebook"));
        assert_eq!(path_of(&ctx, &e), None);
    }

    #[test]
    fn folder_rename_of_a_nested_folder_leaves_the_parent() {
        let (_tmp, ctx) = test_context();
        let parent = create_at(&ctx, "Index", Some("notes"));
        let child = create_at(&ctx, "Ownership", Some("notes/rust"));

        folder_rename(&ctx, "notes/rust", "notes/rustlang").unwrap();

        assert_eq!(path_of(&ctx, &parent).as_deref(), Some("notes"));
        assert_eq!(path_of(&ctx, &child).as_deref(), Some("notes/rustlang"));
    }

    #[test]
    fn folder_rename_into_an_existing_folder_merges() {
        let (_tmp, ctx) = test_context();
        let draft = create_at(&ctx, "Draft", Some("drafts"));
        let note = create_at(&ctx, "Note", Some("notes"));

        let report = folder_rename(&ctx, "drafts", "notes").unwrap();

        assert_eq!(report.changed, vec![draft.clone()]);
        assert_eq!(path_of(&ctx, &draft).as_deref(), Some("notes"));
        assert_eq!(path_of(&ctx, &note).as_deref(), Some("notes"));
    }

    #[test]
    fn folder_rename_with_no_pages_changes_nothing() {
        let (_tmp, ctx) = test_context();
        create_at(&ctx, "Note", Some("notes"));
        let report = folder_rename(&ctx, "nothing", "here").unwrap();
        assert!(report.changed.is_empty());
        assert!(report.failed.is_empty());
    }

    #[test]
    fn folder_rename_rejects_bad_prefixes_before_writing() {
        let (_tmp, ctx) = test_context();
        let uuid = create_at(&ctx, "Note", Some("notes"));
        let before = raw_meta(&ctx, &uuid);

        assert!(folder_rename(&ctx, "notes", "").is_err());
        assert!(folder_rename(&ctx, "", "archive").is_err());
        assert!(folder_rename(&ctx, "notes", "/archive").is_err());
        assert!(folder_rename(&ctx, "notes", "a//b").is_err());
        assert_eq!(raw_meta(&ctx, &uuid), before);
    }

    #[cfg(unix)]
    #[test]
    fn folder_rename_reports_a_failing_page_and_rewrites_the_rest() {
        use std::os::unix::fs::PermissionsExt;
        if unsafe { libc_geteuid() } == 0 {
            return; // root ignores directory permissions
        }
        let (_tmp, ctx) = test_context();
        let ok = create_at(&ctx, "Ok", Some("notes"));
        let stuck = create_at(&ctx, "Stuck", Some("notes/deep"));
        let stuck_dir = object_path(&ctx, &stuck);
        std::fs::set_permissions(&stuck_dir, std::fs::Permissions::from_mode(0o555)).unwrap();

        let report = folder_rename(&ctx, "notes", "archive").unwrap();
        std::fs::set_permissions(&stuck_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(report.changed, vec![ok.clone()]);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, stuck);
        assert_eq!(path_of(&ctx, &ok).as_deref(), Some("archive"));
        assert_eq!(path_of(&ctx, &stuck).as_deref(), Some("notes/deep"));
    }

    #[cfg(unix)]
    unsafe fn libc_geteuid() -> u32 {
        unsafe extern "C" {
            fn geteuid() -> u32;
        }
        unsafe { geteuid() }
    }
}
