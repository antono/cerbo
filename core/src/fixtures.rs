use crate::{index, page, vault, CerboContext};
use std::path::Path;
use tempfile::TempDir;

pub struct FixtureVault {
    pub ctx: CerboContext,
    pub tmp_dir: TempDir,
    pub vault_id: String,
    pub vault_path: std::path::PathBuf,
}

/// Create a fixture vault with the following structure:
/// - Page A: Links to "Page B"
/// - Page B: Target of rename (Title: "Page B", Slug: "page-b")
/// - Page C: Links to "page b" (case-insensitive)
/// - Page D: No links to B
/// - Page E: Links to "Page B" and "Page A"
pub fn create_fixture_vault() -> Result<FixtureVault, String> {
    let tmp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let config_dir = tmp_dir.path().join("config");
    let cache_dir = tmp_dir.path().join("cache");
    let vault_path = tmp_dir.path().join("vault");

    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&vault_path).map_err(|e| e.to_string())?;

    let ctx = CerboContext {
        config_dir,
        cache_dir,
    };

    let vault = vault::vault_add(&ctx, "Fixture".into(), vault_path.to_str().unwrap().into())?;
    let vault_id = vault.id;

    // Create Page B (Target)
    page::page_create(&ctx, vault_id.clone(), "Page B".into())?;

    // Create Page A (Links to B)
    let a_slug = page::page_create(&ctx, vault_id.clone(), "Page A".into())?;
    page::page_write(
        &ctx,
        vault_id.clone(),
        a_slug,
        "# Page A\n\nLink to [[Page B]].".into(),
    )?;

    // Create Page C (Links to B case-insensitively)
    let c_slug = page::page_create(&ctx, vault_id.clone(), "Page C".into())?;
    page::page_write(
        &ctx,
        vault_id.clone(),
        c_slug,
        "# Page C\n\nLink to [[page b]].".into(),
    )?;

    // Create Page D (No links to B)
    page::page_create(&ctx, vault_id.clone(), "Page D".into())?;

    // Create Page E (Links to B and A)
    let e_slug = page::page_create(&ctx, vault_id.clone(), "Page E".into())?;
    page::page_write(
        &ctx,
        vault_id.clone(),
        e_slug,
        "# Page E\n\nLinks to [[Page B]] and [[Page A]].".into(),
    )?;

    // Build index initially
    let idx = index::build_index(&vault_path)?;
    index::save_index(&ctx, &vault_id, &idx)?;

    Ok(FixtureVault {
        ctx,
        tmp_dir,
        vault_id,
        vault_path,
    })
}
