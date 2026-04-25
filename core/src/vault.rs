use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::config;
use crate::paths;
use crate::state;
use crate::CerboContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultsFile {
    pub vaults: Vec<Vault>,
}

pub fn load_vaults(ctx: &CerboContext) -> Result<VaultsFile, String> {
    Ok(VaultsFile {
        vaults: config::load_config(ctx)?.vaults,
    })
}

pub fn save_vaults(ctx: &CerboContext, registry: &VaultsFile) -> Result<(), String> {
    let config = config::Config {
        vaults: registry.vaults.clone(),
    };
    config::save_config(ctx, &config)?;
    Ok(())
}

// ── Business Logic ────────────────────────────────────────────────────────────

pub fn vault_add(ctx: &CerboContext, name: String, path: String) -> Result<Vault, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("vault_add: path does not exist: {path}"));
    }
    let mut reg = load_vaults(ctx)?;
    let vault = Vault {
        id: Uuid::new_v4().to_string(),
        name,
        path: path_buf,
    };
    reg.vaults.push(vault.clone());
    save_vaults(ctx, &reg)?;
    let mut st = state::load_state(ctx)?;
    if st.active_vault_id.is_none() {
        st.active_vault_id = Some(vault.id.clone());
        state::save_state(ctx, &st)?;
    }
    Ok(vault)
}

pub fn vault_remove(ctx: &CerboContext, id: String) -> Result<(), String> {
    let mut reg = load_vaults(ctx)?;
    let before = reg.vaults.len();
    reg.vaults.retain(|v| v.id != id);
    if reg.vaults.len() == before {
        return Err(format!("vault_remove: vault not found: {id}"));
    }
    save_vaults(ctx, &reg)?;
    let mut st = state::load_state(ctx)?;
    if st.active_vault_id.as_deref() == Some(&id) {
        st.active_vault_id = reg.vaults.first().map(|v| v.id.clone());
        state::save_state(ctx, &st)?;
    }
    // Best-effort: delete cache dir
    if let Ok(cache) = paths::cache_dir(ctx.cache_dir.clone(), &id) {
        let _ = std::fs::remove_dir_all(cache);
    }
    Ok(())
}

pub fn vault_list(ctx: &CerboContext) -> Result<VaultsFile, String> {
    load_vaults(ctx)
}

pub fn vault_set_active(ctx: &CerboContext, id: String) -> Result<(), String> {
    let reg = load_vaults(ctx)?;
    if !reg.vaults.iter().any(|v| v.id == id) {
        return Err(format!("vault_set_active: vault not found: {id}"));
    }
    save_vaults(ctx, &reg)?;
    let mut st = state::load_state(ctx)?;
    st.active_vault_id = Some(id);
    state::save_state(ctx, &st)
}

pub fn vault_update_last_page(
    ctx: &CerboContext,
    vault_id: String,
    slug: Option<String>,
) -> Result<(), String> {
    let reg = load_vaults(ctx)?;
    let vault = reg
        .vaults
        .iter()
        .find(|v| v.id == vault_id)
        .ok_or_else(|| format!("vault_update_last_page: vault not found: {vault_id}"))?;
    let mut st = state::load_state(ctx)?;
    st.vault_states
        .entry(vault.id.clone())
        .or_default()
        .last_open_page = slug;
    state::save_state(ctx, &st)
}

pub fn vault_relocate(ctx: &CerboContext, id: String, new_path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&new_path);
    if !path_buf.exists() {
        return Err(format!("vault_relocate: path does not exist: {new_path}"));
    }
    let mut reg = load_vaults(ctx)?;
    let vault = reg
        .vaults
        .iter_mut()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("vault_relocate: vault not found: {id}"))?;
    vault.path = path_buf;
    save_vaults(ctx, &reg)
}

/// Helper used by the watcher and rename cascade: look up the filesystem path
/// for a vault by its ID.
pub fn get_vault_path(ctx: &CerboContext, vault_id: &str) -> Option<PathBuf> {
    load_vaults(ctx)
        .ok()?
        .vaults
        .into_iter()
        .find(|v| v.id == vault_id)
        .map(|v| v.path)
}

pub fn vault_page_count(ctx: &CerboContext, vault_id: &str) -> Result<usize, String> {
    let root = vault_root(ctx, vault_id)?;
    let count = WalkDir::new(&root)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .filter(|e| e.path().join("page.md").exists())
        .count();
    Ok(count)
}

fn vault_root(ctx: &CerboContext, vault_id: &str) -> Result<PathBuf, String> {
    let reg = load_vaults(ctx)?;
    let vault = reg
        .vaults
        .iter()
        .find(|v| v.id == vault_id)
        .ok_or_else(|| format!("vault not found: {vault_id}"))?;
    Ok(vault.path.clone())
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state;
    use std::fs;
    use tempfile::TempDir;

    fn make_vaults_file(dir: &TempDir) -> VaultsFile {
        VaultsFile {
            vaults: vec![Vault {
                id: "a".into(),
                name: "Test".into(),
                path: dir.path().to_path_buf(),
            }],
        }
    }

    #[test]
    fn round_trip_vaults_file() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("vaults.toml");
        let reg = make_vaults_file(&tmp);
        let toml = toml::to_string_pretty(&config::Config { vaults: reg.vaults }).unwrap();
        fs::write(&p, &toml).unwrap();
        let loaded = load_vaults(&CerboContext {
            config_dir: tmp.path().to_path_buf(),
            cache_dir: tmp.path().to_path_buf(),
        })
        .unwrap();
        assert_eq!(loaded.vaults.len(), 1);
        assert_eq!(loaded.vaults[0].id, "a");
    }

    #[test]
    fn vault_set_active_persists_to_state() {
        let tmp = TempDir::new().unwrap();
        let vault_dir = tmp.path().join("vault");
        fs::create_dir_all(&vault_dir).unwrap();
        let ctx = CerboContext {
            config_dir: tmp.path().join("config"),
            cache_dir: tmp.path().join("cache"),
        };
        let vault =
            vault_add(&ctx, "Test".into(), vault_dir.to_string_lossy().to_string()).unwrap();

        vault_set_active(&ctx, vault.id.clone()).unwrap();

        let loaded = state::load_state(&ctx).unwrap();
        assert_eq!(loaded.active_vault_id.as_deref(), Some(vault.id.as_str()));
    }

    #[test]
    fn vault_update_last_page_persists_to_state() {
        let tmp = TempDir::new().unwrap();
        let vault_dir = tmp.path().join("vault");
        fs::create_dir_all(&vault_dir).unwrap();
        let ctx = CerboContext {
            config_dir: tmp.path().join("config"),
            cache_dir: tmp.path().join("cache"),
        };
        let vault =
            vault_add(&ctx, "Test".into(), vault_dir.to_string_lossy().to_string()).unwrap();

        vault_update_last_page(&ctx, vault.id.clone(), Some("page-slug".into())).unwrap();

        let loaded = state::load_state(&ctx).unwrap();
        assert_eq!(
            loaded
                .vault_states
                .get(&vault.id)
                .and_then(|s| s.last_open_page.as_deref()),
            Some("page-slug")
        );
    }
}
