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
