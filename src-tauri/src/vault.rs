use crate::get_context;
use cerbo_core::vault::{self, Vault, VaultsFile, VaultObject};
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
    uuid: Option<String>,
) -> Result<(), String> {
    vault::vault_update_last_page(&get_context(&app)?, vaultId, uuid)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn vault_relocate(app: AppHandle, id: String, newPath: String) -> Result<(), String> {
    vault::vault_relocate(&get_context(&app)?, id, newPath)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn vault_objects_list(app: AppHandle, vaultId: String) -> Result<Vec<VaultObject>, String> {
    let ctx = get_context(&app)?;
    let vault_path = vault::get_vault_path(&ctx, &vaultId)
        .ok_or_else(|| format!("vault not found: {}", vaultId))?;
    vault::list_vault_objects(&ctx, &vault_path)
}

// ── Import of an Obsidian or Tolaria vault ───────────────────────────────────

use cerbo_core::vault_import::{
    self, Detection, Format, ImportOptions, ImportReport, ReportVault,
};
use cerbo_core::CerboContext;
use serde::Deserialize;
use std::path::PathBuf;

/// Where an import writes: a registered vault, or one to create and register.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportTarget {
    Existing { id: String },
    New { path: String, name: String },
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|e| e.to_string())?
}

/// Which format the folder looks like, so the dialog can warn on a mismatch.
/// `None` when it is neither (or a Cerbo vault, which is an error).
#[tauri::command]
pub async fn vault_import_detect(source: String) -> Result<Option<Detection>, String> {
    blocking(move || import_detect(&source)).await
}

/// The dry run: reads the source, writes nothing.
#[tauri::command]
pub async fn vault_import_plan(source: String, format: Format) -> Result<ImportReport, String> {
    blocking(move || Ok(vault_import::plan_import(&PathBuf::from(source), Some(format))?.report())).await
}

/// Import for real. Every failure that can be known up front comes before the
/// first write, in the same order as `cerbo import-vault`: plan, target checks,
/// then (for a new vault) create + register, then the import itself.
fn import_run(
    ctx: &CerboContext,
    source: &str,
    format: Format,
    target: ImportTarget,
    force: bool,
    on_progress: &mut dyn FnMut(vault_import::Progress),
) -> Result<ImportReport, String> {
    let plan = vault_import::plan_import(&PathBuf::from(source), Some(format))?;
    let vault = match target {
        ImportTarget::Existing { id } => {
            let v = vault::vault_list(ctx)?
                .vaults
                .into_iter()
                .find(|v| v.id == id)
                .ok_or_else(|| format!("Vault not found: {id}"))?;
            vault_import::check_target_outside_source(&plan.source, &v.path)?;
            if !v.path.join(".cerbo").is_dir() {
                return Err(format!("Not a Cerbo vault (no .cerbo/ directory): {}", v.path.display()));
            }
            if !force {
                let vault_ctx = cerbo_core::VaultContext { vault_path: v.path.clone(), global: ctx.clone() };
                vault_import::check_repeat(&vault_ctx, &plan)?;
            }
            v
        }
        ImportTarget::New { path, name } => {
            let path = PathBuf::from(path);
            vault::check_new_vault_dir(&path)?;
            vault_import::check_target_outside_source(&plan.source, &path)?;
            vault::vault_create(ctx, &path, name, true)?.0
        }
    };
    let vault_ctx = cerbo_core::VaultContext { vault_path: vault.path.clone(), global: ctx.clone() };
    let mut report = vault_import::run_import_with_progress(&vault_ctx, &plan, ImportOptions { force }, on_progress)?;
    let path = report.vault.as_ref().map(|v| v.path.clone()).unwrap_or_else(|| vault.path.display().to_string());
    report.vault = Some(ReportVault { id: Some(vault.id), name: Some(vault.name), path });
    Ok(report)
}

#[tauri::command]
pub async fn vault_import_run(
    app: AppHandle,
    source: String,
    format: Format,
    target: ImportTarget,
    force: bool,
) -> Result<ImportReport, String> {
    use tauri::Emitter;
    let ctx = get_context(&app)?;
    blocking(move || {
        let mut emit = |p: vault_import::Progress| {
            let _ = app.emit("vault-import-progress", p);
        };
        import_run(&ctx, &source, format, target, force, &mut emit)
    })
    .await
}

/// Detection as the command reports it: `None` for a folder that is neither format.
fn import_detect(source: &str) -> Result<Option<Detection>, String> {
    match vault_import::detect(&PathBuf::from(source), None) {
        Ok(d) => Ok(Some(d)),
        Err(e) if e == vault_import::UNDETECTABLE => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use cerbo_core::CerboContext;
    use tempfile::TempDir;

    fn fixture(name: &str) -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../core/tests/fixtures/import")
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    fn ctx(tmp: &TempDir) -> CerboContext {
        CerboContext { config_dir: tmp.path().join("config"), cache_dir: tmp.path().join("cache") }
    }

    fn new_target(tmp: &TempDir, dir: &str) -> ImportTarget {
        ImportTarget::New { path: tmp.path().join(dir).to_string_lossy().into_owned(), name: dir.into() }
    }

    #[test]
    fn detects_each_format_and_reports_none_for_a_plain_folder() {
        assert_eq!(import_detect(&fixture("obsidian")).unwrap().unwrap().format, Format::Obsidian);
        assert_eq!(import_detect(&fixture("tolaria")).unwrap().unwrap().format, Format::Tolaria);
        let plain = TempDir::new().unwrap();
        assert!(import_detect(plain.path().to_str().unwrap()).unwrap().is_none());
    }

    #[test]
    fn plan_is_a_dry_run_that_writes_nothing() {
        let report = vault_import::plan_import(&PathBuf::from(fixture("obsidian")), Some(Format::Obsidian))
            .unwrap()
            .report();
        assert!(report.dry_run);
        assert!(report.vault.is_none());
        assert!(report.pages > 0);
    }

    #[test]
    fn imports_into_a_new_vault_and_registers_it() {
        let tmp = TempDir::new().unwrap();
        let ctx = ctx(&tmp);
        let report = import_run(&ctx, &fixture("obsidian"), Format::Obsidian, new_target(&tmp, "v"), false, &mut |_| {}).unwrap();
        assert!(!report.dry_run);
        let v = report.vault.unwrap();
        assert_eq!(v.name.as_deref(), Some("v"));
        assert!(vault::vault_list(&ctx).unwrap().vaults.iter().any(|x| Some(&x.id) == v.id.as_ref()));
    }

    #[test]
    fn progress_runs_through_staging_committing_and_indexing() {
        let tmp = TempDir::new().unwrap();
        let mut seen: Vec<vault_import::Progress> = Vec::new();
        import_run(&ctx(&tmp), &fixture("obsidian"), Format::Obsidian, new_target(&tmp, "v"), false, &mut |p| seen.push(p)).unwrap();
        let phases: Vec<&str> = seen.iter().map(|p| p.phase).collect();
        let first = |name| phases.iter().position(|p| *p == name).unwrap();
        assert!(first("staging") < first("committing") && first("committing") < first("indexing"));
        let last_stage = seen.iter().rfind(|p| p.phase == "staging").unwrap();
        assert_eq!(last_stage.done, last_stage.total);
        let last_commit = seen.iter().rfind(|p| p.phase == "committing").unwrap();
        assert_eq!(last_commit.done, last_commit.total);
        assert_eq!(phases.last(), Some(&"indexing"));
    }

    #[test]
    fn repeat_import_is_refused_unless_forced() {
        let tmp = TempDir::new().unwrap();
        let ctx = ctx(&tmp);
        let first = import_run(&ctx, &fixture("obsidian"), Format::Obsidian, new_target(&tmp, "v"), false, &mut |_| {}).unwrap();
        let id = first.vault.unwrap().id.unwrap();
        let again = |force| {
            import_run(&ctx, &fixture("obsidian"), Format::Obsidian, ImportTarget::Existing { id: id.clone() }, force, &mut |_| {})
        };
        assert!(again(false).is_err());
        assert!(again(true).is_ok());
    }

    #[test]
    fn target_inside_the_source_is_refused_before_any_write() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(src.join(".obsidian")).unwrap();
        std::fs::write(src.join("a.md"), "hi").unwrap();
        let inside = src.join("vault");
        let target = ImportTarget::New { path: inside.to_string_lossy().into_owned(), name: "x".into() };
        let err = import_run(&ctx(&tmp), src.to_str().unwrap(), Format::Obsidian, target, false, &mut |_| {}).unwrap_err();
        assert!(err.contains("inside the source"), "{err}");
        assert!(!inside.exists());
    }

    #[test]
    fn unknown_vault_id_and_non_empty_new_folder_fail() {
        let tmp = TempDir::new().unwrap();
        let ctx = ctx(&tmp);
        let err = import_run(&ctx, &fixture("obsidian"), Format::Obsidian, ImportTarget::Existing { id: "nope".into() }, false, &mut |_| {})
            .unwrap_err();
        assert!(err.contains("Vault not found"), "{err}");
        let full = tmp.path().join("full");
        std::fs::create_dir_all(&full).unwrap();
        std::fs::write(full.join("x"), "").unwrap();
        let err = import_run(&ctx, &fixture("obsidian"), Format::Obsidian, new_target(&tmp, "full"), false, &mut |_| {}).unwrap_err();
        assert!(err.contains("not empty"), "{err}");
    }
}
