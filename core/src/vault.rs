use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::paths;
use crate::CerboContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub last_open_page: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultsFile {
    pub vaults: Vec<Vault>,
    pub active_vault_id: Option<String>,
}

fn vaults_path(ctx: &CerboContext) -> Result<PathBuf, String> {
    Ok(paths::config_dir(ctx.config_dir.clone())?.join("vaults.json"))
}

pub fn load_vaults(ctx: &CerboContext) -> Result<VaultsFile, String> {
    let p = vaults_path(ctx)?;
    if !p.exists() {
        return Ok(VaultsFile::default());
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| format!("load_vaults read: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("load_vaults parse: {e}"))
}

pub fn save_vaults(ctx: &CerboContext, registry: &VaultsFile) -> Result<(), String> {
    let p = vaults_path(ctx)?;
    let tmp = p.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(registry)
        .map_err(|e| format!("save_vaults serialize: {e}"))?;
    std::fs::write(&tmp, json).map_err(|e| format!("save_vaults write tmp: {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("save_vaults rename: {e}"))?;
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
        last_open_page: None,
    };
    reg.vaults.push(vault.clone());
    if reg.active_vault_id.is_none() {
        reg.active_vault_id = Some(vault.id.clone());
    }
    save_vaults(ctx, &reg)?;
    Ok(vault)
}

pub fn vault_remove(ctx: &CerboContext, id: String) -> Result<(), String> {
    let mut reg = load_vaults(ctx)?;
    let before = reg.vaults.len();
    reg.vaults.retain(|v| v.id != id);
    if reg.vaults.len() == before {
        return Err(format!("vault_remove: vault not found: {id}"));
    }
    if reg.active_vault_id.as_deref() == Some(&id) {
        reg.active_vault_id = reg.vaults.first().map(|v| v.id.clone());
    }
    save_vaults(ctx, &reg)?;
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
    let mut reg = load_vaults(ctx)?;
    if !reg.vaults.iter().any(|v| v.id == id) {
        return Err(format!("vault_set_active: vault not found: {id}"));
    }
    reg.active_vault_id = Some(id);
    save_vaults(ctx, &reg)
}

pub fn vault_update_last_page(
    ctx: &CerboContext,
    vault_id: String,
    slug: Option<String>,
) -> Result<(), String> {
    let mut reg = load_vaults(ctx)?;
    let vault = reg
        .vaults
        .iter_mut()
        .find(|v| v.id == vault_id)
        .ok_or_else(|| format!("vault_update_last_page: vault not found: {vault_id}"))?;
    vault.last_open_page = slug;
    save_vaults(ctx, &reg)
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
    use std::fs;
    use tempfile::TempDir;

    fn make_vaults_file(dir: &TempDir) -> VaultsFile {
        VaultsFile {
            vaults: vec![Vault {
                id: "a".into(),
                name: "Test".into(),
                path: dir.path().to_path_buf(),
                last_open_page: None,
            }],
            active_vault_id: Some("a".into()),
        }
    }

    #[test]
    fn round_trip_vaults_file() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("vaults.json");
        let reg = make_vaults_file(&tmp);
        let json = serde_json::to_string_pretty(&reg).unwrap();
        fs::write(&p, &json).unwrap();
        let loaded: VaultsFile = serde_json::from_str(&fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(loaded.vaults.len(), 1);
        assert_eq!(loaded.vaults[0].id, "a");
        assert_eq!(loaded.active_vault_id.as_deref(), Some("a"));
    }
}
