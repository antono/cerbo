mod index;
mod page;
mod paths;
mod rename;
mod slug;
mod vault;

use cerbo_core::context::CoreContext;
use cerbo_core::CerboContext;
use index::WatcherState;

pub fn get_context(_app: &tauri::AppHandle) -> Result<CerboContext, String> {
    let core = CoreContext::new()?;
    Ok(CerboContext {
        config_dir: core.config_dir,
        cache_dir: core.cache_dir,
    })
}

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
            page::attachment_list,
            page::attachment_add,
            page::attachment_upload,
            page::attachment_delete,
            page::attachment_open,
            app_exit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn app_exit(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
#[allow(non_snake_case)]
fn vault_open(
    app: tauri::AppHandle,
    vaultId: String,
    watcher_state: tauri::State<WatcherState>,
) -> Result<(), String> {
    let ctx = get_context(&app)?;
    let vault_path = cerbo_core::vault::get_vault_path(&ctx, &vaultId)
        .ok_or_else(|| format!("vault_open: vault not found: {vaultId}"))?;

    // Build/refresh index if cache is missing
    if cerbo_core::index::load_index(&ctx, &vaultId).is_none() {
        let idx = cerbo_core::index::build_index(&vault_path)?;
        cerbo_core::index::save_index(&ctx, &vaultId, &idx)?;
    }

    // Start FS watcher
    index::start_watcher(&app, &vaultId, vault_path, &watcher_state)?;
    Ok(())
}
