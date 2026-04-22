use serde::Serialize;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::{slug::derive_slug, vault::load_vaults};
use crate::CerboContext;

#[derive(Debug, Serialize)]
pub struct PageMeta {
    pub slug: String,
    pub title: String,
}

pub fn vault_root(ctx: &CerboContext, vault_id: &str) -> Result<PathBuf, String> {
    let reg = load_vaults(ctx)?;
    let vault = reg
        .vaults
        .iter()
        .find(|v| v.id == vault_id)
        .ok_or_else(|| format!("page: vault not found: {vault_id}"))?;
    Ok(vault.path.clone())
}

pub fn page_path(root: &PathBuf, slug: &str) -> PathBuf {
    root.join(slug).join("page.md")
}

// ── Business Logic ────────────────────────────────────────────────────────────

pub fn page_create(ctx: &CerboContext, vault_id: String, title: String) -> Result<String, String> {
    let root = vault_root(ctx, &vault_id)?;
    let slug = derive_slug(&title);
    if slug.is_empty() {
        return Err("page_create: title produces empty slug".into());
    }
    let dir = root.join(&slug);
    if dir.exists() {
        return Err(format!("page_create: slug already exists: {slug}"));
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("page_create mkdir: {e}"))?;
    let content = format!("# {title}\n");
    std::fs::write(dir.join("page.md"), content)
        .map_err(|e| format!("page_create write: {e}"))?;
    Ok(slug)
}

pub fn page_read(ctx: &CerboContext, vault_id: String, slug: String) -> Result<String, String> {
    let root = vault_root(ctx, &vault_id)?;
    let p = page_path(&root, &slug);
    std::fs::read_to_string(&p).map_err(|e| format!("page_read: {e}"))
}

pub fn page_write(
    ctx: &CerboContext,
    vault_id: String,
    slug: String,
    content: String,
) -> Result<(), String> {
    let root = vault_root(ctx, &vault_id)?;
    let p = page_path(&root, &slug);
    if !p.parent().map(|d| d.exists()).unwrap_or(false) {
        return Err(format!("page_write: page dir does not exist: {slug}"));
    }
    let tmp = p.with_extension("md.tmp");
    std::fs::write(&tmp, &content).map_err(|e| format!("page_write write tmp: {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("page_write rename: {e}"))?;
    Ok(())
}

pub fn page_delete(ctx: &CerboContext, vault_id: String, slug: String) -> Result<(), String> {
    let root = vault_root(ctx, &vault_id)?;
    let dir = root.join(&slug);
    if !dir.exists() {
        return Err(format!("page_delete: page not found: {slug}"));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("page_delete: {e}"))
}

pub fn page_list(ctx: &CerboContext, vault_id: String) -> Result<Vec<PageMeta>, String> {
    let root = vault_root(ctx, &vault_id)?;
    let mut pages = Vec::new();
    for entry in WalkDir::new(&root).min_depth(1).max_depth(1) {
        let entry = entry.map_err(|e| format!("page_list walk: {e}"))?;
        if !entry.file_type().is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        let page_md = entry.path().join("page.md");
        if !page_md.exists() {
            continue;
        }
        // Extract title from first line starting with "# "
        let title = extract_title(&page_md).unwrap_or_else(|| slug.clone());
        pages.push(PageMeta { slug, title });
    }
    pages.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(pages)
}

pub fn extract_title(path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            return Some(title.trim().to_string());
        }
    }
    None
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a minimal fake vault directory and a Vault entry manually.
    fn setup_vault() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        (tmp, path)
    }

    fn create_page_manually(root: &PathBuf, slug: &str, title: &str) {
        let dir = root.join(slug);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("page.md"), format!("# {title}\n")).unwrap();
    }

    #[test]
    fn create_read_delete() {
        let (tmp, root) = setup_vault();
        let slug = "rust-ownership";
        // create
        let dir = root.join(slug);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("page.md"), "# Rust Ownership\n").unwrap();
        assert!(dir.join("page.md").exists());
        // read
        let content = fs::read_to_string(dir.join("page.md")).unwrap();
        assert!(content.contains("Rust Ownership"));
        // delete
        fs::remove_dir_all(&dir).unwrap();
        assert!(!dir.exists());
        drop(tmp);
    }

    #[test]
    fn list_pages() {
        let (tmp, root) = setup_vault();
        create_page_manually(&root, "alpha", "Alpha");
        create_page_manually(&root, "beta", "Beta");
        // A dir without page.md should be ignored
        fs::create_dir_all(root.join("ignored")).unwrap();

        let mut pages: Vec<PageMeta> = WalkDir::new(&root)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
            .filter(|e| e.path().join("page.md").exists())
            .map(|e| {
                let slug = e.file_name().to_string_lossy().to_string();
                let title = extract_title(&e.path().join("page.md")).unwrap_or(slug.clone());
                PageMeta { slug, title }
            })
            .collect();
        pages.sort_by(|a, b| a.slug.cmp(&b.slug));
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].slug, "alpha");
        assert_eq!(pages[1].title, "Beta");
        drop(tmp);
    }
}
