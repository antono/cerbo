mod index;
mod page;
mod paths;
mod rename;
mod slug;
mod vault;

use index::WatcherState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(WatcherState::default())
        .invoke_handler(tauri::generate_handler![
            // vault
            vault::vault_add,
            vault::vault_remove,
            vault::vault_list,
            vault::vault_set_active,
            vault::vault_relocate,
            vault::vault_update_last_page,
            // index
            index::backlinks_get,
            vault_open,
            // rename
            rename::page_rename,
            // slug
            slug::slug_from_title,
            // page
            page::page_create,
            page::page_read,
            page::page_write,
            page::page_delete,
            page::page_list,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Open a vault: (re)build the link index if missing, start the FS watcher.
/// Call this whenever the user switches to a vault.
#[tauri::command]
fn vault_open(
    app: tauri::AppHandle,
    vault_id: String,
    watcher_state: tauri::State<WatcherState>,
) -> Result<(), String> {
    let vault_path = vault::get_vault_path(&app, &vault_id)
        .ok_or_else(|| format!("vault_open: vault not found: {vault_id}"))?;

    // Build/refresh index if cache is missing
    if index::load_index(&app, &vault_id).is_none() {
        let idx = index::build_index(&vault_path)?;
        index::save_index(&app, &vault_id, &idx)?;
    }

    // Start FS watcher
    index::start_watcher(&app, &vault_id, vault_path, &watcher_state)?;
    Ok(())
}
