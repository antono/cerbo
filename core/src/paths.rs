use std::path::PathBuf;

/// Returns the vault configuration directory.
pub fn config_dir(base_config: PathBuf) -> Result<PathBuf, String> {
    std::fs::create_dir_all(&base_config).map_err(|e| format!("config_dir mkdir: {e}"))?;
    Ok(base_config)
}

/// Returns the cache directory for a specific vault.
pub fn cache_dir(base_cache: PathBuf, vault_id: &str) -> Result<PathBuf, String> {
    let dir = base_cache.join(vault_id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cache_dir mkdir: {e}"))?;
    Ok(dir)
}
