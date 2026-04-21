use crate::index;
use crate::slug::derive_slug;
use crate::vault;
use std::path::Path;
use tauri::AppHandle;

// ---------------------------------------------------------------------------
// Rename cascade
// ---------------------------------------------------------------------------

/// Rename a page: derive a new slug from `new_title`, rename the folder,
/// update all `[[OldTitle]]` links across the vault, and rebuild the index.
#[tauri::command]
pub fn page_rename(
    app: AppHandle,
    vault_id: String,
    old_slug: String,
    new_title: String,
) -> Result<String, String> {
    let vault_path = vault::get_vault_path(&app, &vault_id)
        .ok_or_else(|| format!("page_rename: vault not found: {vault_id}"))?;

    let new_slug = derive_slug(&new_title);

    // 7.2 Reject if new slug already exists (and is different)
    if new_slug != old_slug {
        let new_dir = vault_path.join(&new_slug);
        if new_dir.exists() {
            return Err(format!(
                "page_rename: slug already exists: {new_slug}"
            ));
        }
    } else if new_slug == old_slug {
        // Title changed but slug unchanged — still update the heading
    }

    // Retrieve old title from page before rename (for link rewriting)
    let old_page = vault_path.join(&old_slug).join("page.md");
    let old_content = std::fs::read_to_string(&old_page)
        .map_err(|e| format!("page_rename: read old page: {e}"))?;
    let old_title = extract_title_from_content(&old_content, &old_slug);

    // 7.3 Rename folder
    if new_slug != old_slug {
        let old_dir = vault_path.join(&old_slug);
        let new_dir = vault_path.join(&new_slug);
        std::fs::rename(&old_dir, &new_dir)
            .map_err(|e| format!("page_rename: rename folder: {e}"))?;
    }

    // Update the heading in the renamed page
    let target_page = vault_path.join(&new_slug).join("page.md");
    let new_content = rewrite_heading(&old_content, &new_title);
    std::fs::write(&target_page, &new_content)
        .map_err(|e| format!("page_rename: write new heading: {e}"))?;

    // 7.4 Cascade: rewrite [[OldTitle]] → [[NewTitle]] in all other page.md files
    rewrite_links_in_vault(&vault_path, &old_title, &new_title, &new_slug)?;

    // 7.5 Rebuild link index
    let idx = index::build_index(&vault_path)?;
    index::save_index(&app, &vault_id, &idx)?;

    Ok(new_slug)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_title_from_content(content: &str, fallback: &str) -> String {
    for line in content.lines() {
        if let Some(heading) = line.trim().strip_prefix("# ") {
            return heading.trim().to_string();
        }
    }
    fallback.to_string()
}

/// Replace the first `# Heading` line with `# NewTitle`.
fn rewrite_heading(content: &str, new_title: &str) -> String {
    let mut replaced = false;
    let lines: Vec<String> = content
        .lines()
        .map(|line| {
            if !replaced && line.trim().starts_with("# ") {
                replaced = true;
                format!("# {new_title}")
            } else {
                line.to_string()
            }
        })
        .collect();
    if !replaced {
        // Prepend heading if none existed
        let mut out = format!("# {new_title}\n\n");
        out.push_str(content);
        return out;
    }
    lines.join("\n")
}

/// Walk vault, replace `[[old_title]]` (case-insensitive) with `[[new_title]]`
/// in all `page.md` files. Skip the page we just renamed.
fn rewrite_links_in_vault(
    vault_path: &Path,
    old_title: &str,
    new_title: &str,
    skip_slug: &str,
) -> Result<(), String> {
    let entries = std::fs::read_dir(vault_path)
        .map_err(|e| format!("rewrite_links: read_dir: {e}"))?;

    for entry in entries.flatten() {
        let slug_dir = entry.path();
        if !slug_dir.is_dir() {
            continue;
        }
        let slug = match slug_dir.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        if slug == skip_slug {
            continue;
        }
        let page_file = slug_dir.join("page.md");
        if !page_file.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&page_file)
            .map_err(|e| format!("rewrite_links: read {}: {e}", page_file.display()))?;
        let new_content = replace_wikilink(&content, old_title, new_title);
        if new_content != content {
            std::fs::write(&page_file, new_content)
                .map_err(|e| format!("rewrite_links: write {}: {e}", page_file.display()))?;
        }
    }
    Ok(())
}

/// Case-insensitively replace `[[old_title]]` with `[[new_title]]` in `text`.
fn replace_wikilink(text: &str, old_title: &str, new_title: &str) -> String {
    let old_lower = old_title.to_lowercase();
    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("[[") {
        result.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        if let Some(end) = rest.find("]]") {
            let inner = &rest[..end];
            if inner.trim().to_lowercase() == old_lower {
                result.push_str("[[");
                result.push_str(new_title);
                result.push_str("]]");
            } else {
                result.push_str("[[");
                result.push_str(inner);
                result.push_str("]]");
            }
            rest = &rest[end + 2..];
        } else {
            // Unclosed `[[` — emit as-is
            result.push_str("[[");
            break;
        }
    }
    result.push_str(rest);
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_wikilink_basic() {
        let out = replace_wikilink("See [[Old Title]] here.", "Old Title", "New Title");
        assert_eq!(out, "See [[New Title]] here.");
    }

    #[test]
    fn replace_wikilink_case_insensitive() {
        let out = replace_wikilink("Link [[old title]] end.", "Old Title", "New Title");
        assert_eq!(out, "Link [[New Title]] end.");
    }

    #[test]
    fn replace_wikilink_no_match() {
        let out = replace_wikilink("See [[Other]] here.", "Old Title", "New Title");
        assert_eq!(out, "See [[Other]] here.");
    }

    #[test]
    fn replace_wikilink_multiple() {
        let out = replace_wikilink(
            "[[Old Title]] and [[Old Title]] again.",
            "Old Title",
            "New Title",
        );
        assert_eq!(out, "[[New Title]] and [[New Title]] again.");
    }

    #[test]
    fn rewrite_heading_replaces_first_h1() {
        let content = "# Old Title\n\nSome text.\n";
        let out = rewrite_heading(content, "New Title");
        assert!(out.starts_with("# New Title"));
        assert!(out.contains("Some text."));
    }

    #[test]
    fn rename_no_links() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let vault = dir.path();

        // Create page: rust/page.md
        let rust_dir = vault.join("rust");
        std::fs::create_dir_all(&rust_dir).unwrap();
        std::fs::write(rust_dir.join("page.md"), "# Rust\n\nContent.").unwrap();

        // Rename folder manually (simulating what page_rename does)
        let new_slug = derive_slug("Rust Lang");
        let new_dir = vault.join(&new_slug);
        std::fs::rename(&rust_dir, &new_dir).unwrap();

        // Rewrite heading
        let page_file = new_dir.join("page.md");
        let content = std::fs::read_to_string(&page_file).unwrap();
        let new_content = rewrite_heading(&content, "Rust Lang");
        std::fs::write(&page_file, &new_content).unwrap();

        let saved = std::fs::read_to_string(&page_file).unwrap();
        assert!(saved.starts_with("# Rust Lang"));
        assert!(!vault.join("rust").exists());
        assert!(new_dir.exists());
    }

    #[test]
    fn rename_with_links() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let vault = dir.path();

        // Create two pages
        let rust_dir = vault.join("rust");
        std::fs::create_dir_all(&rust_dir).unwrap();
        std::fs::write(rust_dir.join("page.md"), "# Rust\n\nContent.").unwrap();

        let svelte_dir = vault.join("svelte");
        std::fs::create_dir_all(&svelte_dir).unwrap();
        std::fs::write(svelte_dir.join("page.md"), "# Svelte\n\nSee [[Rust]].").unwrap();

        // Cascade rewrite
        rewrite_links_in_vault(vault, "Rust", "Rust Lang", "rust").unwrap();

        let svelte_content = std::fs::read_to_string(svelte_dir.join("page.md")).unwrap();
        assert!(svelte_content.contains("[[Rust Lang]]"));
        assert!(!svelte_content.contains("[[Rust]]"));
    }

    #[test]
    fn rename_conflict_detected() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let vault = dir.path();

        // Two slugs already exist
        std::fs::create_dir_all(vault.join("rust")).unwrap();
        std::fs::create_dir_all(vault.join("rust-lang")).unwrap();

        let old_slug = "rust";
        let new_slug = derive_slug("Rust Lang");
        assert_eq!(new_slug, "rust-lang");

        // The conflict check in page_rename logic
        let conflict = vault.join(&new_slug).exists() && new_slug != old_slug;
        assert!(conflict);
    }
}
