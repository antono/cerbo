use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Returns `$XDG_CONFIG_HOME/cerbo/` (created on first use).
pub fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("config_dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("config_dir mkdir: {e}"))?;
    Ok(dir)
}

/// Returns `$XDG_CACHE_HOME/cerbo/<vault_id>/` (created on first use).
pub fn cache_dir(app: &AppHandle, vault_id: &str) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("cache_dir: {e}"))?;
    let dir = base.join(vault_id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cache_dir mkdir: {e}"))?;
    Ok(dir)
}
