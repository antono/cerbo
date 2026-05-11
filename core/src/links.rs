use crate::{CerboContext, VaultContext, object};
use regex::Regex;
use std::fs;
use std::path::PathBuf;

// ── Link Extraction ─────────────────────────────────────

/// Extract cerbo://<uuid> links from page content
pub fn extract_cerbo_links(content: &str) -> Vec<String> {
    let re = Regex::new(r"cerbo://([a-z0-9-]+)").unwrap();
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

fn backrefs_read_from_path(obj_dir: &PathBuf) -> Result<Vec<String>, String> {
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
    write_backrefs_to_path(&obj_dir, backrefs)
}

/// Write backrefs.ttl (vault-aware)
fn write_backrefs_vault(vault_ctx: &VaultContext, uuid: &str, backrefs: &[String]) -> Result<(), String> {
    let obj_dir = vault_ctx.object_path(uuid);
    write_backrefs_to_path(&obj_dir, backrefs)
}

fn write_backrefs_to_path(obj_dir: &PathBuf, backrefs: &[String]) -> Result<(), String> {
    let backrefs_path = obj_dir.join("backrefs.ttl");

    let mut lines = vec![
        "@prefix : <cerbo://ontology/> .".to_string(),
        "".to_string(),
        "<cerbo://objects/<uuid>>".to_string(),
    ];

    for source_uuid in backrefs {
        lines.push(format!("    :hasBacklink <cerbo://objects/{}> ;", source_uuid));
    }

    if backrefs.is_empty() {
        lines.push("    :hasBacklink <cerbo://objects/none> .".to_string());
    } else {
        // Replace last semicolon with period
        if let Some(last) = lines.last_mut() {
            *last = last.replace(" ;", " .");
        }
    }

    let content = lines.join("\n") + "\n";

    fs::write(&backrefs_path, content)
        .map_err(|e| format!("Failed to write backrefs.ttl: {}", e))
}

/// Parse backrefs.ttl content to extract source UUIDs
fn parse_backrefs(content: &str) -> Result<Vec<String>, String> {
    let mut backrefs = Vec::new();
    let re = Regex::new(r"cerbo://objects/([a-f0-9-]+)").unwrap();

    for line in content.lines() {
        if line.contains(":hasBacklink") {
            if let Some(cap) = re.captures(line) {
                let uuid = cap[1].to_string();
                if uuid != "none" {
                    backrefs.push(uuid);
                }
            }
        }
    }

    Ok(backrefs)
}

// ── Page Write with Link Extraction ──────────────────────────────

/// Wrapper around page_write that also updates backrefs.ttl
/// Call this instead of page_write() when you want link tracking
pub fn page_write_with_links(ctx: &CerboContext, uuid: &str, content: &str) -> Result<(), String> {
    // Read old content to find removed links
    let old_content = object::object_read(ctx, uuid).unwrap_or_default();
    let old_links = extract_cerbo_links(&old_content);

    // Write new content
    object::object_write(ctx, uuid, content)?;

    // Extract new links
    let new_links = extract_cerbo_links(content);

    // Update backrefs for all links in the content
    for link_uuid in &new_links {
        backrefs_add(ctx, link_uuid, uuid)?;
    }

    // Remove backrefs for links that were removed
    for old_uuid in &old_links {
        if !new_links.contains(old_uuid) {
            backrefs_remove(ctx, old_uuid, uuid)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object;
    use std::fs;

    fn create_test_context() -> CerboContext {
        let dir = std::env::temp_dir().join("cerbo_test_links");
        let _ = fs::create_dir_all(&dir);
        let _ = fs::create_dir_all(dir.join("objects"));
        CerboContext {
            config_dir: dir.clone(),
            cache_dir: dir.join("cache"),
        }
    }

    fn cleanup(ctx: &CerboContext) {
        let _ = fs::remove_dir_all(&ctx.config_dir);
    }

    #[test]
    fn test_extract_cerbo_links() {
        let content = "Links to cerbo://uuid-1 and cerbo://uuid-2 and another cerbo://uuid-3";
        let links = extract_cerbo_links(content);
        assert_eq!(links.len(), 3);
        assert!(links.contains(&"uuid-1".to_string()));
        assert!(links.contains(&"uuid-2".to_string()));
        assert!(links.contains(&"uuid-3".to_string()));
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
        let ctx = create_test_context();

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

        cleanup(&ctx);
    }

    #[test]
    fn test_backrefs_remove() {
        let ctx = create_test_context();

        let uuid1 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 1".into()).unwrap();
        let uuid2 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 2".into()).unwrap();

        // Add then remove
        backrefs_add(&ctx, &uuid1, &uuid2).unwrap();
        let backrefs = backrefs_read(&ctx, &uuid1).unwrap();
        assert_eq!(backrefs.len(), 1);

        backrefs_remove(&ctx, &uuid1, &uuid2).unwrap();
        let backrefs = backrefs_read(&ctx, &uuid1).unwrap();
        assert_eq!(backrefs.len(), 0);

        cleanup(&ctx);
    }

    #[test]
    fn test_page_write_with_links() {
        let ctx = create_test_context();

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

        cleanup(&ctx);
    }

    #[test]
    fn test_backrefs_ttl_format() {
        let ctx = create_test_context();

        let uuid1 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 1".into()).unwrap();
        let uuid2 = object::object_create(&ctx, None, object::ObjectType::Product, "Page 2".into()).unwrap();

        backrefs_add(&ctx, &uuid1, &uuid2).unwrap();

        let obj_dir = object::object_path(&ctx, &uuid1);
        let backrefs_content = fs::read_to_string(obj_dir.join("backrefs.ttl")).unwrap();

        assert!(backrefs_content.contains(":hasBacklink"));
        assert!(backrefs_content.contains(&uuid2));

        cleanup(&ctx);
    }
}
