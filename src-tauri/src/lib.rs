mod index;
mod page;
mod paths;
mod semantic;
mod vault;
mod window;

use cerbo_core::context::CoreContext;
use cerbo_core::CerboContext;
use index::WatcherState;
use semantic::IndexGate;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

/// Holds the active vault's filesystem path. Set on vault_open, used by page commands.
pub struct ActiveVaultState(pub Mutex<Option<PathBuf>>);

impl Default for ActiveVaultState {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

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

/// Returns a CerboContext scoped to the active vault's .cerbo/ directory.
/// Page operations (create/read/write/delete/list) must use this so they
/// operate on the current vault rather than the global config dir.
pub fn get_vault_ctx(app: &tauri::AppHandle) -> Result<CerboContext, String> {
    let global = get_context(app)?;
    let state = app.state::<ActiveVaultState>();
    let lock = state.0.lock().map_err(|_| "vault state lock poisoned".to_string())?;
    let vault_path = lock.as_ref().ok_or("No vault is open")?;
    Ok(CerboContext {
        config_dir: vault_path.join(".cerbo"),
        cache_dir: global.cache_dir,
    })
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
    lastSparqlQuery: Option<String>,
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
        lastSparqlQuery,
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
        None,
    );
    cerbo_core::ui_settings::save_ui_settings(&ctx, &merged)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(WatcherState::default())
        .manage(ActiveVaultState::default())
        .manage(IndexGate::default())
        .setup(|app| {
            // A missing or unreadable size only means the default one.
            if let Err(e) = window::restore_size(app.handle()) {
                eprintln!("Warning: could not restore the window size: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // vault
            vault::vault_add,
            vault::vault_remove,
            vault::vault_list,
            vault::vault_set_active,
            vault::vault_relocate,
            vault::vault_update_last_page,
            vault::vault_objects_list,
            // index
            index::backlinks_get,
            // semantic
            semantic::similar_get,
            semantic::embeddings_refresh,
            semantic::prefix_map_get,
            semantic::sparql_query,
            vault_open,
            // page
            page::page_create,
            page::page_read,
            page::page_write,
            page::page_delete,
            page::page_list,
            page::page_update_title,
            page::cursor_position_save,
            page::cursor_position_load,
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

/// The one exit path of the close button and `Ctrl+Q`, after the frontend
/// saved its UI settings: keep the window size, then end the process.
#[tauri::command]
fn app_exit(app: tauri::AppHandle) {
    if let Err(e) = window::save_size(&app) {
        eprintln!("Warning: could not save the window size: {e}");
    }
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
    active_vault: tauri::State<ActiveVaultState>,
    watcher_state: tauri::State<WatcherState>,
) -> Result<(), String> {
    let ctx = get_context(&app)?;
    let vault_path = cerbo_core::vault::get_vault_path(&ctx, &vaultId)
        .ok_or_else(|| format!("vault_open: vault not found: {}", vaultId))?;

    // Record the active vault path so page commands use the right .cerbo/objects/ dir.
    *active_vault.0.lock().map_err(|_| "vault state lock poisoned")? = Some(vault_path.clone());

    // Start FS watcher
    index::start_watcher(&app, &vaultId, vault_path, &watcher_state)?;
    Ok(())
}
