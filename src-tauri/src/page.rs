use crate::get_context;
use cerbo_core::object::{self, ObjectType};
use tauri::AppHandle;

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_create(app: AppHandle, title: String) -> Result<String, String> {
    let ctx = get_context(&app)?;
    object::object_create(&ctx, None, ObjectType::Product, title)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_read(app: AppHandle, uuid: String) -> Result<String, String> {
    let ctx = get_context(&app)?;
    object::object_read(&ctx, &uuid)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_write(
    app: AppHandle,
    uuid: String,
    content: String,
) -> Result<String, String> {
    let ctx = get_context(&app)?;
    object::object_write(&ctx, &uuid, &content)?;
    Ok(content)
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
pub fn page_delete(app: AppHandle, uuid: String) -> Result<(), String> {
    let ctx = get_context(&app)?;
    object::object_delete(&ctx, &uuid)
}

#[derive(serde::Serialize)]
pub struct PageMeta {
    pub uuid: String,
    pub title: String,
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn page_list(app: AppHandle) -> Result<Vec<PageMeta>, String> {
    let ctx = get_context(&app)?;
    let objects_dir = object::objects_dir(&ctx);
    let mut pages = Vec::new();

    if !objects_dir.exists() {
        return Ok(pages);
    }

    let entries = std::fs::read_dir(&objects_dir)
        .map_err(|e| format!("Failed to read objects dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let uuid = entry.file_name().to_string_lossy().to_string();
        let obj_dir = entry.path();

        if !obj_dir.is_dir() {
            continue;
        }

        let meta_path = obj_dir.join("meta.ttl");
        if meta_path.exists() {
            let meta = object::ObjectMeta::read_from_file(&meta_path)
                .map_err(|e| format!("Failed to read meta.ttl: {}", e))?;

            // Only include Product type (pages)
            if matches!(meta.object_type, object::ObjectType::Product) {
                pages.push(PageMeta {
                    uuid: uuid.clone(),
                    title: meta.title,
                });
            }
        }
    }

    Ok(pages)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_list(
    app: AppHandle,
    uuid: String,
) -> Result<Vec<String>, String> {
    let ctx = get_context(&app)?;
    object::attachment_list(&ctx, &uuid)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_add(
    app: AppHandle,
    uuid: String,
    srcPath: std::path::PathBuf,
) -> Result<String, String> {
    let ctx = get_context(&app)?;
    object::attachment_add(&ctx, &uuid, &srcPath)
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
pub fn attachment_delete(
    app: AppHandle,
    uuid: String,
) -> Result<(), String> {
    let ctx = get_context(&app)?;
    object::attachment_delete(&ctx, &uuid)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn attachment_open(
    app: AppHandle,
    uuid: String,
    filename: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let ctx = get_context(&app)?;
    let obj_dir = object::object_path(&ctx, &uuid);
    let path = obj_dir.join(&filename);
    app.opener()
        .open_path(path.to_string_lossy(), None::<String>)
        .map_err(|e| e.to_string())
}
