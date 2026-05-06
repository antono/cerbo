use crate::get_context;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

pub struct WatcherState {
    pub watcher: Option<notify::RecommendedWatcher>,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self { watcher: None }
    }
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn backlinks_get(
    app: AppHandle,
    _vaultId: String,
    uuid: String,
) -> Result<Vec<String>, String> {
    let ctx = get_context(&app)?;
    // Get backlinks from backrefs.ttl
    let backlinks = cerbo_core::links::backrefs_read(&ctx, &uuid)?;
    Ok(backlinks)
}

pub fn start_watcher(
    app: &AppHandle,
    _vault_id: &str,
    _vault_path: PathBuf,
    _watcher_state: &State<WatcherState>,
) -> Result<(), String> {
    // TODO: Implement watcher for UUID model
    // For now, just return Ok
    let _ = app;
    Ok(())
}
