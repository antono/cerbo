use cerbo_core::slug;

#[tauri::command]
pub fn slug_from_title(title: String) -> String {
    slug::derive_slug(&title)
}
