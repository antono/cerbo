use crate::paths::cache_dir;
use crate::CerboContext;
use notify::{Config, Event, RecommendedWatcher, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageEntry {
    pub title: String,
    pub links: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkEntry {
    pub slug: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkIndex {
    pub version: u32,
    pub built_at: String,
    pub pages: HashMap<String, PageEntry>,
}

impl LinkIndex {
    fn new(pages: HashMap<String, PageEntry>) -> Self {
        Self {
            version: 1,
            built_at: chrono_now(),
            pages,
        }
    }
}

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ---------------------------------------------------------------------------
// Wikilink extraction
// ---------------------------------------------------------------------------

pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        if let Some(end) = rest.find("]]") {
            let target = rest[..end].trim().to_string();
            if !target.is_empty() {
                links.push(target);
            }
            rest = &rest[end + 2..];
        } else {
            break;
        }
    }
    links
}

pub fn extract_title(content: &str, slug: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            return heading.trim().to_string();
        }
    }
    slug.to_string()
}

// ---------------------------------------------------------------------------
// Build / load / save
// ---------------------------------------------------------------------------

pub fn build_index(vault_path: &Path) -> Result<LinkIndex, String> {
    let mut pages: HashMap<String, PageEntry> = HashMap::new();

    let entries =
        std::fs::read_dir(vault_path).map_err(|e| format!("build_index read_dir: {e}"))?;

    for entry in entries.flatten() {
        let slug_path = entry.path();
        if !slug_path.is_dir() {
            continue;
        }
        let page_file = slug_path.join("page.md");
        if !page_file.exists() {
            continue;
        }
        let slug = match slug_path.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let content = std::fs::read_to_string(&page_file)
            .map_err(|e| format!("build_index read {}: {e}", page_file.display()))?;
        let title = extract_title(&content, &slug);
        let links = extract_wikilinks(&content);
        pages.insert(slug, PageEntry { title, links });
    }

    Ok(LinkIndex::new(pages))
}

pub fn load_index(ctx: &CerboContext, vault_id: &str) -> Option<LinkIndex> {
    let path = index_path(ctx, vault_id).ok()?;
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_index(ctx: &CerboContext, vault_id: &str, index: &LinkIndex) -> Result<(), String> {
    let path = index_path(ctx, vault_id)?;
    let data =
        serde_json::to_string_pretty(index).map_err(|e| format!("save_index serialize: {e}"))?;
    std::fs::write(&path, data).map_err(|e| format!("save_index write: {e}"))?;
    Ok(())
}

fn index_path(ctx: &CerboContext, vault_id: &str) -> Result<PathBuf, String> {
    Ok(cache_dir(ctx.cache_dir.clone(), vault_id)?.join("index.json"))
}

// ---------------------------------------------------------------------------
// Backlinks
// ---------------------------------------------------------------------------

pub fn compute_backlinks(index: &LinkIndex, target_slug: &str) -> Vec<BacklinkEntry> {
    let target_title = index
        .pages
        .get(target_slug)
        .map(|e| e.title.as_str())
        .unwrap_or(target_slug);

    index
        .pages
        .iter()
        .filter_map(|(slug, entry)| {
            let links_to = entry.links.iter().any(|l| {
                l.eq_ignore_ascii_case(target_title) || l.eq_ignore_ascii_case(target_slug)
            });
            if links_to && slug != target_slug {
                Some(BacklinkEntry {
                    slug: slug.clone(),
                    title: entry.title.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// FS watcher state
// ---------------------------------------------------------------------------

pub struct WatcherState(pub Arc<Mutex<Option<RecommendedWatcher>>>);

impl Default for WatcherState {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

pub fn create_watcher<F>(handler: F) -> Result<RecommendedWatcher, String>
where
    F: FnMut(notify::Result<Event>) + Send + 'static,
{
    RecommendedWatcher::new(handler, Config::default()).map_err(|e| format!("watcher create: {e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_wikilinks_basic() {
        let links = extract_wikilinks("See [[Rust]] and [[Svelte]].");
        assert_eq!(links, vec!["Rust", "Svelte"]);
    }

    #[test]
    fn test_extract_wikilinks_empty() {
        let links = extract_wikilinks("No links here.");
        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_wikilinks_unclosed() {
        let links = extract_wikilinks("[[Incomplete");
        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_title_from_heading() {
        let title = extract_title("# My Page\n\nContent.", "my-page");
        assert_eq!(title, "My Page");
    }

    #[test]
    fn test_extract_title_fallback() {
        let title = extract_title("No heading here.", "my-page");
        assert_eq!(title, "my-page");
    }

    #[test]
    fn test_compute_backlinks() {
        let mut pages = HashMap::new();
        pages.insert(
            "rust".to_string(),
            PageEntry {
                title: "Rust".to_string(),
                links: vec!["Svelte".to_string()],
            },
        );
        pages.insert(
            "svelte".to_string(),
            PageEntry {
                title: "Svelte".to_string(),
                links: vec![],
            },
        );
        pages.insert(
            "other".to_string(),
            PageEntry {
                title: "Other".to_string(),
                links: vec!["Svelte".to_string(), "Rust".to_string()],
            },
        );
        let index = LinkIndex::new(pages);

        let mut bl = compute_backlinks(&index, "svelte");
        bl.sort_by_key(|b| b.slug.clone());
        assert_eq!(bl.len(), 2);
        assert_eq!(bl[0].slug, "other");
        assert_eq!(bl[1].slug, "rust");

        let bl2 = compute_backlinks(&index, "rust");
        assert_eq!(bl2.len(), 1);
        assert_eq!(bl2[0].slug, "other");
    }

    #[test]
    fn test_build_index() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let vault = dir.path();

        // Create two pages
        let p1 = vault.join("rust");
        std::fs::create_dir_all(&p1).unwrap();
        std::fs::write(p1.join("page.md"), "# Rust\n\nSee [[Svelte]].").unwrap();

        let p2 = vault.join("svelte");
        std::fs::create_dir_all(&p2).unwrap();
        std::fs::write(p2.join("page.md"), "# Svelte\n\nNo links.").unwrap();

        let index = build_index(vault).unwrap();
        assert_eq!(index.pages.len(), 2);

        let rust_entry = index.pages.get("rust").unwrap();
        assert_eq!(rust_entry.title, "Rust");
        assert_eq!(rust_entry.links, vec!["Svelte"]);

        let svelte_entry = index.pages.get("svelte").unwrap();
        assert_eq!(svelte_entry.title, "Svelte");
        assert!(svelte_entry.links.is_empty());
    }

    #[test]
    fn test_built_at_is_portable_timestamp() {
        let index = LinkIndex::new(HashMap::new());
        assert!(!index.built_at.starts_with("SystemTime {"));
        assert!(index.built_at.contains('T'));
    }
}
