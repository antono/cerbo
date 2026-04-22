use crate::index;
use crate::slug::derive_slug;
use crate::vault;
use crate::CerboContext;
use std::path::Path;

// ---------------------------------------------------------------------------
// Rename cascade
// ---------------------------------------------------------------------------

/// Rename a page: derive a new slug from `new_title`, rename the folder,
/// update all `[[OldTitle]]` links across the vault, and rebuild the index.
pub fn page_rename(
    ctx: &CerboContext,
    vault_id: String,
    old_slug: String,
    new_title: String,
) -> Result<String, String> {
    let vault_path = vault::get_vault_path(ctx, &vault_id)
        .ok_or_else(|| format!("page_rename: vault not found: {vault_id}"))?;

    let new_slug = derive_slug(&new_title);

    // Reject if new slug already exists (and is different)
    if new_slug != old_slug {
        let new_dir = vault_path.join(&new_slug);
        if new_dir.exists() {
            return Err(format!(
                "page_rename: slug already exists: {new_slug}"
            ));
        }
    }

    // Retrieve old title from page before rename (for link rewriting)
    let old_page = vault_path.join(&old_slug).join("page.md");
    let old_content = std::fs::read_to_string(&old_page)
        .map_err(|e| format!("page_rename: read old page: {e}"))?;
    let old_title = extract_title_from_content(&old_content, &old_slug);

    // Rename folder
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

    // Cascade: rewrite [[OldTitle]] → [[NewTitle]] in all other page.md files
    let index = index::load_index(ctx, &vault_id);
    let affected_slugs = if let Some(ref idx) = index {
        index::compute_backlinks(idx, &old_slug)
            .into_iter()
            .map(|b| b.slug)
            .collect::<Vec<_>>()
    } else {
        // Fallback to full vault scan if index is missing
        println!("Warning: No index found for vault {vault_id}, falling back to full scan.");
        get_all_slugs(&vault_path)?
    };

    rewrite_links_in_pages(&vault_path, &affected_slugs, &old_title, &new_title, &new_slug)?;

    // Rebuild link index
    let idx = index::build_index(&vault_path)?;
    index::save_index(ctx, &vault_id, &idx)?;

    Ok(new_slug)
}

fn get_all_slugs(vault_path: &Path) -> Result<Vec<String>, String> {
    let mut slugs = Vec::new();
    let entries = std::fs::read_dir(vault_path)
        .map_err(|e| format!("get_all_slugs: read_dir: {e}"))?;
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            if let Some(s) = entry.file_name().to_str() {
                slugs.push(s.to_string());
            }
        }
    }
    Ok(slugs)
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

/// Update `[[old_title]]` (case-insensitive) with `[[new_title]]`
/// in the specified pages. Skip the page we just renamed.
fn rewrite_links_in_pages(
    vault_path: &Path,
    slugs: &[String],
    old_title: &str,
    new_title: &str,
    skip_slug: &str,
) -> Result<(), String> {
    for slug in slugs {
        if slug == skip_slug {
            continue;
        }
        let slug_dir = vault_path.join(slug);
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
    fn test_rename_cascade_smart() {
        use crate::fixtures::create_fixture_vault;
        use std::fs;

        let fixture = create_fixture_vault().unwrap();
        
        // Rename Page B -> New Page B
        let new_slug = page_rename(&fixture.ctx, fixture.vault_id.clone(), "page-b".into(), "New Page B".into()).unwrap();
        assert_eq!(new_slug, "new-page-b");

        // Verify Page A (Linked to Page B)
        let a_content = fs::read_to_string(fixture.vault_path.join("page-a").join("page.md")).unwrap();
        assert!(a_content.contains("[[New Page B]]"));
        assert!(!a_content.contains("[[Page B]]"));

        // Verify Page C (Linked to page b - case insensitive)
        let c_content = fs::read_to_string(fixture.vault_path.join("page-c").join("page.md")).unwrap();
        assert!(c_content.contains("[[New Page B]]"));
        assert!(!c_content.contains("[[page b]]"));

        // Verify Page E (Linked to Page B and Page A)
        let e_content = fs::read_to_string(fixture.vault_path.join("page-e").join("page.md")).unwrap();
        assert!(e_content.contains("[[New Page B]]"));
        assert!(e_content.contains("[[Page A]]"));

        // Verify Page D (No links)
        let d_content = fs::read_to_string(fixture.vault_path.join("page-d").join("page.md")).unwrap();
        assert!(!d_content.contains("New Page B"));

        // Verify Index updated
        let idx = index::load_index(&fixture.ctx, &fixture.vault_id).unwrap();
        assert!(idx.pages.contains_key("new-page-b"));
        assert!(!idx.pages.contains_key("page-b"));
        
        let a_entry = idx.pages.get("page-a").unwrap();
        assert!(a_entry.links.contains(&"New Page B".to_string()));
    }
}
