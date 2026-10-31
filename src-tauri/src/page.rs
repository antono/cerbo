use crate::get_vault_ctx;
use cerbo_core::object;
use tauri::AppHandle;

/// Run a command that changes vault files off the main thread. Each such
/// change also updates the semantic index, which can wait up to the index
/// lock's timeout while e.g. "Enable" holds it; on the main thread that wait
/// would freeze the window (spec: the similar notes panel never blocks editing).
/// The frontend sends these one at a time, so saves keep their order.
async fn off_main<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn page_create(app: AppHandle, title: String, slug: Option<String>, virtualPath: Option<String>) -> Result<String, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::page::page_create_with_metadata(&ctx, title, slug, virtualPath)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_read(app: AppHandle, uuid: String) -> Result<String, String> {
    let ctx = get_vault_ctx(&app)?;
    cerbo_core::page::page_read(&ctx, uuid)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn page_write(
    app: AppHandle,
    uuid: String,
    content: String,
) -> Result<String, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::page::page_write(&ctx, uuid, content)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn page_update_title(
    app: AppHandle,
    uuid: String,
    newTitle: String,
) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::page::page_update_title(&ctx, uuid, newTitle)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn cursor_position_save(
    _app: AppHandle,
    _uuid: String,
    _line: u32,
    _column: u32,
) -> Result<(), String> {
    // TODO: Implement cursor position save for UUID model
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn cursor_position_load(
    _app: AppHandle,
    _uuid: String,
) -> Result<Option<(u32, u32)>, String> {
    // TODO: Implement cursor position load for UUID model
    Ok(None)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn page_delete(app: AppHandle, uuid: String) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || cerbo_core::page::page_delete(&ctx, uuid)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_list(app: AppHandle) -> Result<Vec<cerbo_core::page::PageMeta>, String> {
    let ctx = get_vault_ctx(&app)?;
    cerbo_core::page::page_list(&ctx)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_list(
    app: AppHandle,
    uuid: String,
) -> Result<Vec<String>, String> {
    let ctx = get_vault_ctx(&app)?;
    object::attachment_list(&ctx, &uuid)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn attachment_add(
    app: AppHandle,
    uuid: String,
    srcPath: std::path::PathBuf,
) -> Result<String, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || object::attachment_add(&ctx, &uuid, &srcPath)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_upload(
    _app: AppHandle,
    _uuid: String,
    _filename: String,
    _data: Vec<u8>,
) -> Result<String, String> {
    // TODO: Implement attachment upload for UUID model
    Err("Not implemented yet".to_string())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn attachment_delete(
    app: AppHandle,
    uuid: String,
) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || object::attachment_delete(&ctx, &uuid)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_open(
    app: AppHandle,
    uuid: String,
    filename: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let ctx = get_vault_ctx(&app)?;
    let obj_dir = object::object_path(&ctx, &uuid);
    let path = obj_dir.join(&filename);
    app.opener()
        .open_path(path.to_string_lossy(), None::<String>)
        .map_err(|e| e.to_string())
}
