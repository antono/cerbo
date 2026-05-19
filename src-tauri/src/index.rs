use crate::get_vault_ctx;
use cerbo_core::object::ObjectMeta;
use std::path::PathBuf;
use tauri::{AppHandle, State};

pub struct WatcherState {
    #[allow(dead_code)]
    pub watcher: Option<notify::RecommendedWatcher>,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self { watcher: None }
    }
}

#[derive(serde::Serialize)]
pub struct BacklinkEntry {
    pub uuid: String,
    pub title: String,
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn backlinks_get(
    app: AppHandle,
    _vaultId: String,
    uuid: String,
) -> Result<Vec<BacklinkEntry>, String> {
    let ctx = get_vault_ctx(&app)?;
    let backref_uuids = cerbo_core::links::backrefs_read(&ctx, &uuid)?;
    let mut entries = Vec::new();
    for backref_uuid in backref_uuids {
        let obj_dir = cerbo_core::object::object_path(&ctx, &backref_uuid);
        let meta_path = obj_dir.join("meta.ttl");
        if let Ok(meta) = ObjectMeta::read_from_file(&meta_path) {
            entries.push(BacklinkEntry { uuid: backref_uuid, title: meta.title });
        }
    }
    Ok(entries)
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
