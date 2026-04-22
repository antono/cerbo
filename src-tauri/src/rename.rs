use cerbo_core::rename;
use tauri::AppHandle;
use crate::get_context;

#[tauri::command]
pub fn page_rename(
    app: AppHandle,
    vault_id: String,
    old_slug: String,
    new_title: String,
) -> Result<String, String> {
    rename::page_rename(&get_context(&app)?, vault_id, old_slug, new_title)
}
