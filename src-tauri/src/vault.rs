use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;
use uuid::Uuid;

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultsFile {
    pub vaults: Vec<Vault>,
    pub active_vault_id: Option<String>,
}

fn vaults_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(paths::config_dir(app)?.join("vaults.json"))
}

pub fn load_vaults(app: &AppHandle) -> Result<VaultsFile, String> {
    let p = vaults_path(app)?;
    if !p.exists() {
        return Ok(VaultsFile::default());
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| format!("load_vaults read: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("load_vaults parse: {e}"))
}

pub fn save_vaults(app: &AppHandle, registry: &VaultsFile) -> Result<(), String> {
    let p = vaults_path(app)?;
    let tmp = p.with_extension("json.tmp");
    let json =
        serde_json::to_string_pretty(registry).map_err(|e| format!("save_vaults serialize: {e}"))?;
    std::fs::write(&tmp, json).map_err(|e| format!("save_vaults write tmp: {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("save_vaults rename: {e}"))?;
    Ok(())
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn vault_add(app: AppHandle, name: String, path: String) -> Result<Vault, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("vault_add: path does not exist: {path}"));
    }
    let mut reg = load_vaults(&app)?;
    let vault = Vault {
        id: Uuid::new_v4().to_string(),
        name,
        path: path_buf,
    };
    reg.vaults.push(vault.clone());
    if reg.active_vault_id.is_none() {
        reg.active_vault_id = Some(vault.id.clone());
    }
    save_vaults(&app, &reg)?;
    Ok(vault)
}

#[tauri::command]
pub fn vault_remove(app: AppHandle, id: String) -> Result<(), String> {
    let mut reg = load_vaults(&app)?;
    let before = reg.vaults.len();
    reg.vaults.retain(|v| v.id != id);
    if reg.vaults.len() == before {
        return Err(format!("vault_remove: vault not found: {id}"));
    }
    if reg.active_vault_id.as_deref() == Some(&id) {
        reg.active_vault_id = reg.vaults.first().map(|v| v.id.clone());
    }
    save_vaults(&app, &reg)?;
    // Best-effort: delete cache dir
    if let Ok(cache) = paths::cache_dir(&app, &id) {
        let _ = std::fs::remove_dir_all(cache);
    }
    Ok(())
}

#[tauri::command]
pub fn vault_list(app: AppHandle) -> Result<Vec<Vault>, String> {
    Ok(load_vaults(&app)?.vaults)
}

#[tauri::command]
pub fn vault_set_active(app: AppHandle, id: String) -> Result<(), String> {
    let mut reg = load_vaults(&app)?;
    if !reg.vaults.iter().any(|v| v.id == id) {
        return Err(format!("vault_set_active: vault not found: {id}"));
    }
    reg.active_vault_id = Some(id);
    save_vaults(&app, &reg)
}

#[tauri::command]
pub fn vault_relocate(app: AppHandle, id: String, new_path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&new_path);
    if !path_buf.exists() {
        return Err(format!("vault_relocate: path does not exist: {new_path}"));
    }
    let mut reg = load_vaults(&app)?;
    let vault = reg
        .vaults
        .iter_mut()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("vault_relocate: vault not found: {id}"))?;
    vault.path = path_buf;
    save_vaults(&app, &reg)
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
