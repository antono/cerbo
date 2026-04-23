use cerbo_core::rename;
use tauri::AppHandle;
use crate::get_context;

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_rename(
    app: AppHandle,
    vaultId: String,
    oldSlug: String,
    newTitle: String,
    content: Option<String>,
) -> Result<String, String> {
    rename::page_rename(&get_context(&app)?, vaultId, oldSlug, newTitle, content)
}
