use crate::get_context;
use cerbo_core::page::{self, PageMeta};
use tauri::AppHandle;

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_create(app: AppHandle, vaultId: String, title: String) -> Result<String, String> {
    page::page_create(&get_context(&app)?, vaultId, title)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_read(app: AppHandle, vaultId: String, slug: String) -> Result<String, String> {
    page::page_read(&get_context(&app)?, vaultId, slug)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_write(
    app: AppHandle,
    vaultId: String,
    slug: String,
    content: String,
) -> Result<String, String> {
    page::page_write(&get_context(&app)?, vaultId, slug, content)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_delete(app: AppHandle, vaultId: String, slug: String) -> Result<(), String> {
    page::page_delete(&get_context(&app)?, vaultId, slug)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_list(app: AppHandle, vaultId: String) -> Result<Vec<PageMeta>, String> {
    page::page_list(&get_context(&app)?, vaultId)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_list(
    app: AppHandle,
    vaultId: String,
    slug: String,
) -> Result<Vec<String>, String> {
    page::attachment_list(&get_context(&app)?, vaultId, slug)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_add(
    app: AppHandle,
    vaultId: String,
    slug: String,
    srcPath: std::path::PathBuf,
) -> Result<String, String> {
    page::attachment_add(&get_context(&app)?, vaultId, slug, srcPath)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_upload(
    app: AppHandle,
    vaultId: String,
    slug: String,
    filename: String,
    data: Vec<u8>,
) -> Result<String, String> {
    page::attachment_upload(&get_context(&app)?, vaultId, slug, filename, data)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_delete(
    app: AppHandle,
    vaultId: String,
    slug: String,
    filename: String,
) -> Result<(), String> {
    page::attachment_delete(&get_context(&app)?, vaultId, slug, filename)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_open(
    app: AppHandle,
    vaultId: String,
    slug: String,
    filename: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = page::attachment_path(&get_context(&app)?, vaultId, slug, filename)?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<String>)
        .map_err(|e| e.to_string())
}
