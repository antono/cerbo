use cerbo_core::page::{self, PageMeta};
use tauri::AppHandle;
use crate::get_context;

#[tauri::command]
pub fn page_create(app: AppHandle, vault_id: String, title: String) -> Result<String, String> {
    page::page_create(&get_context(&app)?, vault_id, title)
}

#[tauri::command]
pub fn page_read(app: AppHandle, vault_id: String, slug: String) -> Result<String, String> {
    page::page_read(&get_context(&app)?, vault_id, slug)
}

#[tauri::command]
pub fn page_write(
    app: AppHandle,
    vault_id: String,
    slug: String,
    content: String,
) -> Result<(), String> {
    page::page_write(&get_context(&app)?, vault_id, slug, content)
}

#[tauri::command]
pub fn page_delete(app: AppHandle, vault_id: String, slug: String) -> Result<(), String> {
    page::page_delete(&get_context(&app)?, vault_id, slug)
}

#[tauri::command]
pub fn page_list(app: AppHandle, vault_id: String) -> Result<Vec<PageMeta>, String> {
    page::page_list(&get_context(&app)?, vault_id)
}
