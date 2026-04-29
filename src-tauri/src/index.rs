use crate::get_context;
pub use cerbo_core::index::WatcherState;
use cerbo_core::index::{self, BacklinkEntry};
use notify::Watcher;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
#[allow(non_snake_case)]
pub fn backlinks_get(
    app: AppHandle,
    vaultId: String,
    slug: String,
) -> Result<Vec<BacklinkEntry>, String> {
    let ctx = get_context(&app)?;
    let index =
        index::load_index(&ctx, &vaultId).ok_or_else(|| format!("No index for vault {vaultId}"))?;
    Ok(index::compute_backlinks(&index, &slug))
}

pub fn start_watcher(
    app: &AppHandle,
    vault_id: &str,
    vault_path: PathBuf,
    watcher_state: &State<WatcherState>,
) -> Result<(), String> {
    let app_handle = app.clone();
    let vid = vault_id.to_string();

    let handler = move |result: notify::Result<notify::Event>| {
        if let Ok(event) = result {
            let affects_page = event
                .paths
                .iter()
                .any(|p| p.file_name().map(|n| n == "page.md").unwrap_or(false));
            if !affects_page {
                return;
            }
            if get_context(&app_handle).is_ok() {
                for path in &event.paths {
                    let _ = app_handle.emit(
                        "page-file-changed",
                        serde_json::json!({
                            "vaultId": vid,
                            "path": path.to_string_lossy(),
                        }),
                    );
                }
            }
        }
    };

    let watcher = index::create_watcher(handler)?;
    let mut watcher = watcher;
    watcher
        .watch(&vault_path, notify::RecursiveMode::Recursive)
        .map_err(|e| format!("watcher watch: {e}"))?;

    let mut guard = watcher_state
        .0
        .lock()
        .map_err(|e| format!("watcher lock: {e}"))?;
    *guard = Some(watcher);
    Ok(())
}
