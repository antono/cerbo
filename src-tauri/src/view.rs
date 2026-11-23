//! Commands for views (spec `vault-views`): alternative sidebar trees over the
//! vault's pages. The logic is `cerbo_core::view`; this only supplies the
//! SPARQL runner for query nodes and takes the index gate around it.

use crate::semantic::IndexGate;
use crate::{get_context, get_vault_ctx};
use cerbo_core::semantic::{self, IndexOptions};
use cerbo_core::view::{self, ViewMeta, ViewNode, ViewTree};
use cerbo_core::VaultContext;
use tauri::{AppHandle, Manager};

/// Run a view command off the main thread: queries and index updates can wait
/// on the index lock.
async fn off_main<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|e| e.to_string())?
}

/// The open vault for the semantic index; its `.cerbo/` parent is the vault.
fn index_context(app: &AppHandle) -> Result<VaultContext, String> {
    let ctx = get_vault_ctx(app)?;
    let vault_path = ctx.config_dir.parent().unwrap_or(&ctx.config_dir).to_path_buf();
    Ok(VaultContext { vault_path, global: get_context(app)? })
}

/// Runs a query node's SELECT. The caller holds the [`IndexGate`] for the whole
/// command, so this never takes it again.
fn queries(index: VaultContext) -> impl Fn(&str) -> Result<Vec<String>, String> {
    move |sparql| {
        let output = semantic::query(&index, sparql, IndexOptions::default()).map_err(|e| e.to_string())?;
        view::pages_from_output(&output)
    }
}

#[tauri::command]
pub async fn view_list(app: AppHandle) -> Result<Vec<ViewMeta>, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || view::view_list(&ctx)).await
}

#[tauri::command]
pub async fn view_get(app: AppHandle, uuid: String) -> Result<ViewTree, String> {
    let ctx = get_vault_ctx(&app)?;
    let index = index_context(&app)?;
    off_main(move || {
        let gate = app.state::<IndexGate>();
        let _turn = gate.hold();
        view::view_get(&ctx, &uuid, &queries(index))
    })
    .await
}

#[tauri::command]
pub async fn view_create(app: AppHandle, title: String, nodes: Vec<ViewNode>) -> Result<String, String> {
    let ctx = get_vault_ctx(&app)?;
    let index = index_context(&app)?;
    off_main(move || {
        let gate = app.state::<IndexGate>();
        let _turn = gate.hold();
        view::view_create(&ctx, &title, &nodes, &queries(index))
    })
    .await
}

#[tauri::command]
pub async fn view_update(app: AppHandle, uuid: String, title: Option<String>, nodes: Vec<ViewNode>) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    let index = index_context(&app)?;
    off_main(move || {
        let gate = app.state::<IndexGate>();
        let _turn = gate.hold();
        view::view_update(&ctx, &uuid, title.as_deref(), &nodes, &queries(index))
    })
    .await
}

#[tauri::command]
pub async fn view_delete(app: AppHandle, uuid: String) -> Result<(), String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || view::view_delete(&ctx, &uuid)).await
}

/// The stored nodes of a view, for a change that edits the tree in place (the
/// sidebar sends the whole tree back with `view_update`).
#[tauri::command]
pub async fn view_nodes(app: AppHandle, uuid: String) -> Result<Vec<ViewNode>, String> {
    let ctx = get_vault_ctx(&app)?;
    off_main(move || view::view_nodes(&ctx, &uuid)).await
}
