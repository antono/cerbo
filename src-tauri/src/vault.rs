use crate::get_context;
use cerbo_core::vault::{self, Vault, VaultsFile};
use tauri::AppHandle;

#[tauri::command]
pub fn vault_add(app: AppHandle, name: String, path: String) -> Result<Vault, String> {
    vault::vault_add(&get_context(&app)?, name, path)
}

#[tauri::command]
pub fn vault_remove(app: AppHandle, id: String) -> Result<(), String> {
    vault::vault_remove(&get_context(&app)?, id)
}

#[tauri::command]
pub fn vault_list(app: AppHandle) -> Result<VaultsFile, String> {
    vault::vault_list(&get_context(&app)?)
}

#[tauri::command]
pub fn vault_set_active(app: AppHandle, id: String) -> Result<(), String> {
    vault::vault_set_active(&get_context(&app)?, id)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn vault_update_last_page(
    app: AppHandle,
    vaultId: String,
    slug: Option<String>,
) -> Result<(), String> {
    vault::vault_update_last_page(&get_context(&app)?, vaultId, slug)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn vault_relocate(app: AppHandle, id: String, newPath: String) -> Result<(), String> {
    vault::vault_relocate(&get_context(&app)?, id, newPath)
}
