use crate::{index::{self, IndexJson}, object, page, CerboContext};
use std::path::Path;
use tempfile::TempDir;

pub struct FixtureVault {
    pub ctx: CerboContext,
    pub tmp_dir: TempDir,
    pub config_dir: std::path::PathBuf,
}

/// Create a fixture vault with UUID-based objects
/// Creates several pages with links between them
pub fn create_fixture_vault() -> Result<FixtureVault, String> {
    let tmp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let config_dir = tmp_dir.path().join(".cerbo");
    let cache_dir = tmp_dir.path().join("cache");
    let vault_path = tmp_dir.path().join("vault");

    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&vault_path).map_err(|e| e.to_string())?;

    let ctx = CerboContext {
        config_dir: config_dir.clone(),
        cache_dir,
    };

    // Initialize vault structure
    let objects_dir = config_dir.join("objects");
    std::fs::create_dir_all(&objects_dir).map_err(|e| e.to_string())?;

    // Create index.json
    let index = IndexJson::default();
    index::index_save(&ctx, &index)?;

    // Create Page A
    let uuid_a = object::object_create(&ctx, None, object::ObjectType::Product, "Page A".into())?;
    page::page_write(&ctx, uuid_a.clone(), "# Page A\n\nLink to [[Page B]].".into())?;

    // Create Page B (target of links)
    let uuid_b = object::object_create(&ctx, None, object::ObjectType::Product, "Page B".into())?;
    page::page_write(&ctx, uuid_b.clone(), "# Page B\n\nContent here.".into())?;

    // Create Page C (links to B case-insensitively)
    let uuid_c = object::object_create(&ctx, None, object::ObjectType::Product, "Page C".into())?;
    page::page_write(&ctx, uuid_c.clone(), "# Page C\n\nLink to [[page b]].".into())?;

    // Create Page D (no links to B)
    let _uuid_d = object::object_create(&ctx, None, object::ObjectType::Product, "Page D".into())?;

    // Create Page E (links to B and A)
    let uuid_e = object::object_create(&ctx, None, object::ObjectType::Product, "Page E".into())?;
    page::page_write(&ctx, uuid_e.clone(), "# Page E\n\nLinks to [[Page B]] and [[Page A]].".into())?;

    // Update index with all pages
    for (uuid, title) in &[(uuid_a.clone(), "Page A"), (uuid_b.clone(), "Page B"), (uuid_c.clone(), "Page C"), (uuid_e.clone(), "Page E")] {
        index::index_add(&ctx, title, uuid)?;
    }

    Ok(FixtureVault {
        ctx,
        tmp_dir,
        config_dir,
    })
}
