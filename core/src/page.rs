use serde::{Deserialize, Serialize};
use std::path::{PathBuf, Path};
use crate::CerboContext;
use crate::object::{object_path, ObjectMeta};

/// Get the full path to a page object directory within a vault
pub fn get_page_path(vault_path: &Path, page_uuid: &str) -> PathBuf {
    vault_path.join(".cerbo").join("objects").join(page_uuid)
}

#[derive(Debug, Serialize)]
pub struct PageMeta {
    pub uuid: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
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
/// Returns the content on success (for compatibility with old API).
pub fn page_write(
    ctx: &CerboContext,
    uuid: String,
    content: String,
) -> Result<String, String> {
    crate::links::page_write_with_links(ctx, &uuid, &content)?;
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
        let title = ObjectMeta::read_from_file(&path.join("meta.ttl"))
            .ok()
            .map(|meta| meta.title)
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "Untitled".to_string());

        pages.push(PageMeta { uuid, title });
    }

    pages.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(pages)
}

/// Cursor position functions (stubbed for now)
pub fn cursor_position_save(
    _ctx: &CerboContext,
    _uuid: String,
    _line: u32,
    _column: u32,
) -> Result<(), String> {
    // TODO: Implement with cursor position storage
    Ok(())
}

pub fn cursor_position_load(
    _ctx: &CerboContext,
    _uuid: String,
) -> Result<Option<CursorPosition>, String> {
    // TODO: Implement with cursor position storage
    Ok(None)
}

/// Attachment functions (UUID model)
pub fn attachment_list(
    _ctx: &CerboContext,
    _uuid: String,
) -> Result<Vec<String>, String> {
    // TODO: Read backrefs.ttl for :usesAttachment
    Ok(Vec::new())
}

pub fn attachment_add(
    _ctx: &CerboContext,
    page_uuid: String,
    src_path: PathBuf,
) -> Result<String, String> {
    // TODO: Create attachment object, update page's backrefs.ttl
    let _ = (&page_uuid, &src_path);
    Err("Not yet implemented".to_string())
}

pub fn attachment_delete(
    _ctx: &CerboContext,
    _uuid: String,
    _filename: String,
) -> Result<(), String> {
    // TODO: Delete attachment object
    Err("Not yet implemented".to_string())
}

/// Stub functions for tests (to be removed)
pub fn has_h1(_content: &str) -> bool { false }
pub fn humanize_slug(_slug: &str) -> String { String::new() }
pub fn ensure_page_has_h1(_path: &Path, _slug: &str) -> Result<bool, String> { Ok(false) }

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
}
