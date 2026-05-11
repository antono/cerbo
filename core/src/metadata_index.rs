/// Metadata indexing operations for page backrefs and annotations
/// 
/// This module provides functions to rebuild page metadata (backrefs.ttl and annotations.ttl)
/// from page content. Used by the `cerbo index` CLI command.

use crate::VaultContext;

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

/// Index all pages within a specific vault (two-pass within vault scope)
pub fn index_vault(vault_ctx: &VaultContext) -> Result<IndexStats, String> {
    let mut stats = IndexStats::default();
    
    // Phase 1: Get all pages in this vault
    let page_uuids = crate::vault::list_pages_in_vault(&vault_ctx.global, &vault_ctx.vault_path)?;
    
    // Phase 2: Clear all backrefs for pages in this vault
    for uuid in &page_uuids {
        if let Err(e) = crate::links::backrefs_clear_vault(vault_ctx, uuid) {
            stats.errors.push(IndexError {
                page_uuid: uuid.clone(),
                error_message: format!("Failed to clear backrefs: {}", e),
            });
        }
    }
    
    // Phase 3: Rebuild all metadata
    for uuid in page_uuids {
        let page_stats = index_page(vault_ctx, &uuid)?;
        stats.merge(page_stats);
    }
    
    Ok(stats)
}

/// Index a single page (incremental: update only this page's links and annotations)
pub fn index_page(vault_ctx: &VaultContext, page_uuid: &str) -> Result<IndexStats, String> {
    let mut stats = IndexStats::default();
    
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
    
    if let Err(e) = crate::annotations::annotations_write_vault(vault_ctx, page_uuid, &annotations) {
        stats.errors.push(IndexError {
            page_uuid: page_uuid.to_string(),
            error_message: format!("Failed to write annotations: {}", e),
        });
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

#[cfg(test)]
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
        let map = serde_json::json!({"prefixes": {}});
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
