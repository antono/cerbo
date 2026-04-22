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

#[tauri::command]
pub fn attachment_list(
    app: AppHandle,
    vault_id: String,
    slug: String,
) -> Result<Vec<String>, String> {
    page::attachment_list(&get_context(&app)?, vault_id, slug)
}

#[tauri::command]
pub fn attachment_add(
    app: AppHandle,
    vault_id: String,
    slug: String,
    src_path: std::path::PathBuf,
) -> Result<String, String> {
    page::attachment_add(&get_context(&app)?, vault_id, slug, src_path)
}

#[tauri::command]
pub fn attachment_upload(
    app: AppHandle,
    vault_id: String,
    slug: String,
    filename: String,
    data: Vec<u8>,
) -> Result<String, String> {
    page::attachment_upload(&get_context(&app)?, vault_id, slug, filename, data)
}

#[tauri::command]
pub fn attachment_delete(
    app: AppHandle,
    vault_id: String,
    slug: String,
    filename: String,
) -> Result<(), String> {
    page::attachment_delete(&get_context(&app)?, vault_id, slug, filename)
}
