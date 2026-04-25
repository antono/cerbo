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
    let ctx = CerboContext {
        config_dir: core.config_dir,
        cache_dir: core.cache_dir,
    };
    let _ = cerbo_core::migration::migrate_if_needed(&ctx)?;
    if !ctx.config_dir.join("vaults.toml").exists() {
        cerbo_core::config::save_config(&ctx, &cerbo_core::config::Config::default())?;
    }
    if !ctx.config_dir.join("ui.toml").exists() {
        cerbo_core::ui_settings::save_ui_settings(
            &ctx,
            &cerbo_core::ui_settings::UiSettings::default(),
        )?;
    }
    if !ctx.cache_dir.join("state.toml").exists() {
        cerbo_core::state::save_state(&ctx, &cerbo_core::state::State::default())?;
    }
    Ok(ctx)
}

#[tauri::command]
#[allow(non_snake_case)]
fn ui_settings_load(app: tauri::AppHandle) -> Result<cerbo_core::ui_settings::UiSettings, String> {
    let ctx = get_context(&app)?;
    cerbo_core::ui_settings::load_ui_settings(&ctx)
}

#[tauri::command]
#[allow(non_snake_case)]
fn ui_settings_save(
    app: tauri::AppHandle,
    theme: Option<String>,
    fontSize: Option<u8>,
    sidebarWidth: Option<u16>,
    rightSidebarVisible: Option<bool>,
    windowBounds: Option<cerbo_core::ui_settings::WindowBounds>,
) -> Result<(), String> {
    let ctx = get_context(&app)?;
    let current = cerbo_core::ui_settings::load_ui_settings(&ctx)?;
    let merged = cerbo_core::ui_settings::merge_ui_settings(
        &current,
        theme,
        fontSize,
        sidebarWidth,
        rightSidebarVisible,
        windowBounds,
    );
    cerbo_core::ui_settings::save_ui_settings(&ctx, &merged)
}

#[tauri::command]
fn window_bounds_load(
    app: tauri::AppHandle,
) -> Result<Option<cerbo_core::ui_settings::WindowBounds>, String> {
    let ctx = get_context(&app)?;
    Ok(cerbo_core::ui_settings::load_ui_settings(&ctx)?.window_bounds)
}

#[tauri::command]
fn window_bounds_save(app: tauri::AppHandle, width: f64, height: f64) -> Result<(), String> {
    let ctx = get_context(&app)?;
    let current = cerbo_core::ui_settings::load_ui_settings(&ctx)?;
    let merged = cerbo_core::ui_settings::merge_ui_settings(
        &current,
        None,
        None,
        None,
        None,
        Some(cerbo_core::ui_settings::WindowBounds { width, height }),
    );
    cerbo_core::ui_settings::save_ui_settings(&ctx, &merged)
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
            ui_settings_load,
            ui_settings_save,
            window_bounds_load,
            window_bounds_save,
            state_load,
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
fn state_load(app: tauri::AppHandle) -> Result<cerbo_core::state::State, String> {
    let ctx = get_context(&app)?;
    cerbo_core::state::load_state(&ctx)
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
