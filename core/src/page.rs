use serde::{Deserialize, Serialize};
use std::path::{PathBuf, Path};
use crate::CerboContext;

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
    crate::object::object_write(ctx, &uuid, &content)?;
    Ok(content)
}

/// Delete a page by UUID. Fails if type is Source (read-only).
pub fn page_delete(ctx: &CerboContext, uuid: String) -> Result<(), String> {
    crate::object::object_delete(ctx, &uuid)
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
        if !path.is_dir() {
            continue;
        }

        let uuid = entry.file_name().to_string_lossy().to_string();
        let page_md = path.join("page.md");
        if !page_md.exists() {
            continue;
        }

        // Read title from meta.ttl
        let meta_path = path.join("meta.ttl");
        let title = if meta_path.exists() {
            // TODO: Parse meta.ttl properly
            // For now, try to extract from page.md
            std::fs::read_to_string(&page_md)
                .ok()
                .and_then(|content| {
                    content.lines()
                        .find(|l| l.trim().starts_with("# "))
                        .map(|l| l.trim_start_matches("# ").to_string())
                })
                .unwrap_or_else(|| "Untitled".to_string())
        } else {
            "Untitled".to_string()
        };

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
mod tests {
    use super::*;

    #[test]
    fn test_page_list_empty() {
        // TODO: Add proper test with fixtures
    }
}
