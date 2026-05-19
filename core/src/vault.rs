use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::config;
use crate::paths;
use crate::state;
use crate::CerboContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub is_auto: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultsFile {
    pub vaults: Vec<Vault>,
}

pub fn load_vaults(ctx: &CerboContext) -> Result<VaultsFile, String> {
    Ok(VaultsFile {
        vaults: config::load_config(ctx)?.vaults,
    })
}

pub fn save_vaults(ctx: &CerboContext, registry: &VaultsFile) -> Result<(), String> {
    let config = config::Config {
        vaults: registry.vaults.clone(),
    };
    config::save_config(ctx, &config)?;
    Ok(())
}

pub fn load_auto_vaults(ctx: &CerboContext) -> Result<VaultsFile, String> {
    let p = config::auto_config_path(ctx)?;
    if !p.exists() {
        return Ok(VaultsFile::default());
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| format!("load_auto_vaults read: {e}"))?;
    toml::from_str(&raw).map_err(|e| format!("load_auto_vaults parse: {e}"))
}

pub fn save_auto_vaults(ctx: &CerboContext, registry: &VaultsFile) -> Result<(), String> {
    let p = config::auto_config_path(ctx)?;
    let tmp = p.with_extension("toml.tmp");
    let raw = toml::to_string_pretty(registry).map_err(|e| format!("save_auto_vaults serialize: {e}"))?;
    std::fs::write(&tmp, raw).map_err(|e| format!("save_auto_vaults write tmp: {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("save_auto_vaults rename: {e}"))?;
    Ok(())
}

/// List all vaults (convenience wrapper around load_vaults)
pub fn list_all_vaults(ctx: &CerboContext) -> Result<Vec<Vault>, String> {
    Ok(load_vaults(ctx)?.vaults)
}

/// List all page UUIDs in a vault (scans vault_path/.cerbo/objects/)
pub fn list_pages_in_vault(_ctx: &CerboContext, vault_path: &PathBuf) -> Result<Vec<String>, String> {
    let objects_dir = vault_path.join(".cerbo").join("objects");
    
    if !objects_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut page_uuids = Vec::new();
    
    let entries = std::fs::read_dir(&objects_dir)
        .map_err(|e| format!("list_pages_in_vault read_dir: {}", e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("list_pages_in_vault entry: {}", e))?;
        let path = entry.path();
        
        if !path.is_dir() {
            continue;
        }
        
        // Check if it's a page object (has page.md)
        if path.join("page.md").exists() {
            page_uuids.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    
    Ok(page_uuids)
}

// ── Business Logic ────────────────────────────────────────────────────────────

pub fn vault_add(ctx: &CerboContext, name: String, path: String) -> Result<Vault, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("vault_add: path does not exist: {path}"));
    }
    let mut reg = load_vaults(ctx)?;
    let vault = Vault {
        id: Uuid::new_v4().to_string(),
        name,
        path: path_buf,
        is_auto: false,
    };
    reg.vaults.push(vault.clone());
    save_vaults(ctx, &reg)?;
    let mut st = state::load_state(ctx)?;
    if st.active_vault_id.is_none() {
        st.active_vault_id = Some(vault.id.clone());
        state::save_state(ctx, &st)?;
    }
    Ok(vault)
}

pub fn vault_remove(ctx: &CerboContext, id: String) -> Result<(), String> {
    let mut manual = load_vaults(ctx)?;
    let before_manual = manual.vaults.len();
    manual.vaults.retain(|v| v.id != id);

    let mut auto_reg = load_auto_vaults(ctx)?;
    let before_auto = auto_reg.vaults.len();
    auto_reg.vaults.retain(|v| v.id != id);

    if manual.vaults.len() == before_manual && auto_reg.vaults.len() == before_auto {
        return Err(format!("vault_remove: vault not found: {id}"));
    }

    if manual.vaults.len() < before_manual {
        save_vaults(ctx, &manual)?;
    }
    if auto_reg.vaults.len() < before_auto {
        save_auto_vaults(ctx, &auto_reg)?;
    }

    let mut st = state::load_state(ctx)?;
    if st.active_vault_id.as_deref() == Some(&id) {
        st.active_vault_id = vault_list(ctx)
            .ok()
            .and_then(|f| f.vaults.into_iter().next())
            .map(|v| v.id);
        state::save_state(ctx, &st)?;
    }
    if let Ok(cache) = paths::cache_dir(ctx.cache_dir.clone(), &id) {
        let _ = std::fs::remove_dir_all(cache);
    }
    Ok(())
}

pub fn vault_list(ctx: &CerboContext) -> Result<VaultsFile, String> {
    let mut manual = load_vaults(ctx)?;
    for v in &mut manual.vaults {
        v.is_auto = false;
    }

    let auto_file = load_auto_vaults(ctx)?;
    let manual_paths: std::collections::HashSet<std::path::PathBuf> = manual
        .vaults
        .iter()
        .filter_map(|v| std::fs::canonicalize(&v.path).ok())
        .collect();

    for mut v in auto_file.vaults {
        v.is_auto = true;
        let canon = std::fs::canonicalize(&v.path).ok();
        if canon.map_or(true, |c| !manual_paths.contains(&c)) {
            manual.vaults.push(v);
        }
    }

    Ok(manual)
}

pub fn vault_set_active(ctx: &CerboContext, id: String) -> Result<(), String> {
    let reg = load_vaults(ctx)?;
    if !reg.vaults.iter().any(|v| v.id == id) {
        return Err(format!("vault_set_active: vault not found: {id}"));
    }
    save_vaults(ctx, &reg)?;
    let mut st = state::load_state(ctx)?;
    st.active_vault_id = Some(id);
    state::save_state(ctx, &st)
}

pub fn vault_update_last_page(
    ctx: &CerboContext,
    vault_id: String,
    uuid: Option<String>,
) -> Result<(), String> {
    let reg = load_vaults(ctx)?;
    let vault = reg
        .vaults
        .iter()
        .find(|v| v.id == vault_id)
        .ok_or_else(|| format!("vault_update_last_page: vault not found: {vault_id}"))?;
    let mut st = state::load_state(ctx)?;
    st.vault_states
        .entry(vault.id.clone())
        .or_default()
        .last_open_page = uuid;
    state::save_state(ctx, &st)
}

pub fn vault_relocate(ctx: &CerboContext, id: String, new_path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&new_path);
    if !path_buf.exists() {
        return Err(format!("vault_relocate: path does not exist: {new_path}"));
    }
    let mut reg = load_vaults(ctx)?;
    let vault = reg
        .vaults
        .iter_mut()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("vault_relocate: vault not found: {id}"))?;
    vault.path = path_buf;
    save_vaults(ctx, &reg)
}

/// Helper used by the watcher and rename cascade: look up the filesystem path
/// for a vault by its ID.
pub fn get_vault_path(ctx: &CerboContext, vault_id: &str) -> Option<PathBuf> {
    vault_list(ctx)
        .ok()?
        .vaults
        .into_iter()
        .find(|v| v.id == vault_id)
        .map(|v| v.path)
}

pub fn vault_page_count(ctx: &CerboContext, vault_id: &str) -> Result<usize, String> {
    let root = vault_root(ctx, vault_id)?;
    let count = WalkDir::new(&root)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .filter(|e| e.path().join("page.md").exists())
        .count();
    Ok(count)
}

fn vault_root(ctx: &CerboContext, vault_id: &str) -> Result<PathBuf, String> {
    let reg = load_vaults(ctx)?;
    let vault = reg
        .vaults
        .iter()
        .find(|v| v.id == vault_id)
        .ok_or_else(|| format!("vault not found: {vault_id}"))?;
    Ok(vault.path.clone())
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state;
    use std::fs;
    use tempfile::TempDir;

    fn make_vaults_file(dir: &TempDir) -> VaultsFile {
        VaultsFile {
            vaults: vec![Vault {
                id: "a".into(),
                name: "Test".into(),
                path: dir.path().to_path_buf(),
                is_auto: false,
            }],
        }
    }

    #[test]
    fn round_trip_vaults_file() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("vaults.toml");
        let reg = make_vaults_file(&tmp);
        let toml = toml::to_string_pretty(&config::Config { vaults: reg.vaults }).unwrap();
        fs::write(&p, &toml).unwrap();
        let loaded = load_vaults(&CerboContext {
            config_dir: tmp.path().to_path_buf(),
            cache_dir: tmp.path().to_path_buf(),
        })
        .unwrap();
        assert_eq!(loaded.vaults.len(), 1);
        assert_eq!(loaded.vaults[0].id, "a");
    }

    #[test]
    fn vault_set_active_persists_to_state() {
        let tmp = TempDir::new().unwrap();
        let vault_dir = tmp.path().join("vault");
        fs::create_dir_all(&vault_dir).unwrap();
        let ctx = CerboContext {
            config_dir: tmp.path().join("config"),
            cache_dir: tmp.path().join("cache"),
        };
        let vault =
            vault_add(&ctx, "Test".into(), vault_dir.to_string_lossy().to_string()).unwrap();

        vault_set_active(&ctx, vault.id.clone()).unwrap();

        let loaded = state::load_state(&ctx).unwrap();
        assert_eq!(loaded.active_vault_id.as_deref(), Some(vault.id.as_str()));
    }

    #[test]
    fn vault_update_last_page_persists_to_state() {
        let tmp = TempDir::new().unwrap();
        let vault_dir = tmp.path().join("vault");
        fs::create_dir_all(&vault_dir).unwrap();
        let ctx = CerboContext {
            config_dir: tmp.path().join("config"),
            cache_dir: tmp.path().join("cache"),
        };
        let vault =
            vault_add(&ctx, "Test".into(), vault_dir.to_string_lossy().to_string()).unwrap();

        vault_update_last_page(&ctx, vault.id.clone(), Some("page-slug".into())).unwrap();

        let loaded = state::load_state(&ctx).unwrap();
        assert_eq!(
            loaded
                .vault_states
                .get(&vault.id)
                .and_then(|s| s.last_open_page.as_deref()),
            Some("page-slug")
        );
    }

    #[test]
    fn vault_add_persists_active_vault_to_state() {
        let tmp = TempDir::new().unwrap();
        let vault_dir = tmp.path().join("vault");
        fs::create_dir_all(&vault_dir).unwrap();
        let ctx = CerboContext {
            config_dir: tmp.path().join("config"),
            cache_dir: tmp.path().join("cache"),
        };

        let vault =
            vault_add(&ctx, "Test".into(), vault_dir.to_string_lossy().to_string()).unwrap();

        let loaded = state::load_state(&ctx).unwrap();
        assert_eq!(loaded.active_vault_id.as_deref(), Some(vault.id.as_str()));
    }

    // ── vault_list / auto-registry tests ─────────────────────────────────────

    fn make_ctx(tmp: &TempDir) -> CerboContext {
        CerboContext {
            config_dir: tmp.path().join("config"),
            cache_dir: tmp.path().join("cache"),
        }
    }

    #[test]
    fn vault_list_merges_manual_and_auto() {
        let tmp = TempDir::new().unwrap();
        let ctx = make_ctx(&tmp);
        let manual_dir = tmp.path().join("manual");
        let auto_dir = tmp.path().join("auto");
        fs::create_dir_all(&manual_dir).unwrap();
        fs::create_dir_all(&auto_dir).unwrap();

        vault_add(&ctx, "Manual".into(), manual_dir.to_string_lossy().to_string()).unwrap();

        let mut auto_file = VaultsFile {
            vaults: vec![Vault {
                id: "auto-id".into(),
                name: "Auto".into(),
                path: auto_dir.clone(),
                is_auto: false,
            }],
        };
        save_auto_vaults(&ctx, &mut auto_file).unwrap();

        let list = vault_list(&ctx).unwrap();
        assert_eq!(list.vaults.len(), 2);
        let manual_entry = list.vaults.iter().find(|v| v.name == "Manual").unwrap();
        let auto_entry = list.vaults.iter().find(|v| v.name == "Auto").unwrap();
        assert!(!manual_entry.is_auto);
        assert!(auto_entry.is_auto);
    }

    #[test]
    fn vault_list_deduplicates_same_path() {
        let tmp = TempDir::new().unwrap();
        let ctx = make_ctx(&tmp);
        let shared_dir = tmp.path().join("shared");
        fs::create_dir_all(&shared_dir).unwrap();

        vault_add(&ctx, "Manual".into(), shared_dir.to_string_lossy().to_string()).unwrap();

        let auto_file = VaultsFile {
            vaults: vec![Vault {
                id: "auto-id".into(),
                name: "Auto".into(),
                path: shared_dir.clone(),
                is_auto: false,
            }],
        };
        save_auto_vaults(&ctx, &auto_file).unwrap();

        let list = vault_list(&ctx).unwrap();
        assert_eq!(list.vaults.len(), 1, "auto entry with duplicate path should be suppressed");
        assert!(!list.vaults[0].is_auto);
    }

    #[test]
    fn vault_list_no_auto_file_returns_only_manual() {
        let tmp = TempDir::new().unwrap();
        let ctx = make_ctx(&tmp);
        let vault_dir = tmp.path().join("v");
        fs::create_dir_all(&vault_dir).unwrap();

        vault_add(&ctx, "Manual".into(), vault_dir.to_string_lossy().to_string()).unwrap();

        let list = vault_list(&ctx).unwrap();
        assert_eq!(list.vaults.len(), 1);
        assert!(!list.vaults[0].is_auto);
    }

    #[test]
    fn auto_vault_register_adds_entry() {
        let tmp = TempDir::new().unwrap();
        let ctx = make_ctx(&tmp);
        let vault_dir = tmp.path().join("v");
        fs::create_dir_all(&vault_dir).unwrap();

        auto_vault_register(&ctx, &vault_dir).unwrap();

        let auto_reg = load_auto_vaults(&ctx).unwrap();
        assert_eq!(auto_reg.vaults.len(), 1);
        assert_eq!(auto_reg.vaults[0].name, "v");
    }

    #[test]
    fn auto_vault_register_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let ctx = make_ctx(&tmp);
        let vault_dir = tmp.path().join("v");
        fs::create_dir_all(&vault_dir).unwrap();

        auto_vault_register(&ctx, &vault_dir).unwrap();
        auto_vault_register(&ctx, &vault_dir).unwrap();

        let auto_reg = load_auto_vaults(&ctx).unwrap();
        assert_eq!(auto_reg.vaults.len(), 1, "second call must not add duplicate");
    }

    #[test]
    fn auto_vault_register_skips_when_manually_registered() {
        let tmp = TempDir::new().unwrap();
        let ctx = make_ctx(&tmp);
        let vault_dir = tmp.path().join("v");
        fs::create_dir_all(&vault_dir).unwrap();

        vault_add(&ctx, "Manual".into(), vault_dir.to_string_lossy().to_string()).unwrap();
        auto_vault_register(&ctx, &vault_dir).unwrap();

        let auto_reg = load_auto_vaults(&ctx).unwrap();
        assert_eq!(auto_reg.vaults.len(), 0, "should not auto-register an already manually registered path");
    }
}

// ── Repository discovery ──────────────────────────────────────────────────────

/// Walk upward from `start` looking for a directory containing `.cerbo/`.
/// Stops at filesystem mount-point boundaries (Unix: different `st_dev`).
pub fn find_vault_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        if current.join(".cerbo").is_dir() {
            return Some(current);
        }

        let parent = match current.parent() {
            Some(p) if p != current => p.to_path_buf(),
            _ => return None,
        };

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let cur_dev = current.metadata().ok()?.dev();
            let par_dev = parent.metadata().ok()?.dev();
            if cur_dev != par_dev {
                return None;
            }
        }

        current = parent;
    }
}

// ── Auto-registration ─────────────────────────────────────────────────────────

/// Silently record a CWD-discovered vault in `vaults.auto.toml`.
/// Idempotent: no write if path already appears in either registry.
pub fn auto_vault_register(ctx: &CerboContext, root: &Path) -> Result<(), String> {
    let canon = std::fs::canonicalize(root)
        .map_err(|e| format!("auto_vault_register canonicalize: {e}"))?;

    let manual = load_vaults(ctx)?;
    if manual.vaults.iter().any(|v| std::fs::canonicalize(&v.path).ok().as_ref() == Some(&canon)) {
        return Ok(());
    }

    let mut auto_reg = load_auto_vaults(ctx)?;
    if auto_reg.vaults.iter().any(|v| std::fs::canonicalize(&v.path).ok().as_ref() == Some(&canon)) {
        return Ok(());
    }

    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "vault".to_string());

    auto_reg.vaults.push(Vault {
        id: Uuid::new_v4().to_string(),
        name,
        path: root.to_path_buf(),
        is_auto: false,
    });
    save_auto_vaults(ctx, &auto_reg)
}

/// Promote an auto-registered vault to the manual registry.
/// Moves the entry from `vaults.auto.toml` to `vaults.toml`.
pub fn auto_vault_approve(ctx: &CerboContext, id: &str) -> Result<Vault, String> {
    let mut auto_reg = load_auto_vaults(ctx)?;
    let pos = auto_reg
        .vaults
        .iter()
        .position(|v| v.id == id)
        .ok_or_else(|| format!("vault approve: not found in auto registry: {id}"))?;

    let vault = auto_reg.vaults[pos].clone();

    let mut manual = load_vaults(ctx)?;
    let canon = std::fs::canonicalize(&vault.path).ok();
    if manual
        .vaults
        .iter()
        .any(|v| std::fs::canonicalize(&v.path).ok() == canon)
    {
        return Err(format!(
            "vault approve: path already manually registered: {}",
            vault.path.display()
        ));
    }

    let mut approved = vault;
    approved.is_auto = false;
    manual.vaults.push(approved.clone());
    save_vaults(ctx, &manual)?;

    auto_reg.vaults.remove(pos);
    save_auto_vaults(ctx, &auto_reg)?;

    Ok(approved)
}

// ── CWD vault resolution ──────────────────────────────────────────────────────

/// Return the vault ID whose registered path canonicalizes to the same real
/// path as `root`.  Returns `None` when no registered vault matches.
pub fn vault_id_from_path(ctx: &CerboContext, root: &Path) -> Option<String> {
    let canon_root = std::fs::canonicalize(root).ok()?;
    let reg = vault_list(ctx).ok()?;
    reg.vaults.into_iter().find_map(|v| {
        let canon_v = std::fs::canonicalize(&v.path).ok()?;
        if canon_v == canon_root { Some(v.id) } else { None }
    })
}

// ── Virtual path validation ───────────────────────────────────────────────────

/// Error returned when a cerbo:virtualPath value is malformed.
#[derive(Debug, PartialEq)]
pub enum VirtualPathError {
    LeadingOrTrailingSlash,
    DotSegment,
    EmptySegment,
    NulByte,
}

impl std::fmt::Display for VirtualPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LeadingOrTrailingSlash => write!(f, "leading or trailing slash"),
            Self::DotSegment => write!(f, "dot or dotdot segment"),
            Self::EmptySegment => write!(f, "empty segment (double slash)"),
            Self::NulByte => write!(f, "NUL byte"),
        }
    }
}

/// Validate a cerbo:virtualPath string.
/// Empty string (root) is valid. Otherwise: no leading/trailing '/', no '.' or
/// '..' segments, no empty segments, no NUL bytes.
pub fn validate_virtual_path(s: &str) -> Result<(), VirtualPathError> {
    if s.is_empty() {
        return Ok(());
    }
    if s.contains('\0') {
        return Err(VirtualPathError::NulByte);
    }
    if s.starts_with('/') || s.ends_with('/') {
        return Err(VirtualPathError::LeadingOrTrailingSlash);
    }
    for segment in s.split('/') {
        if segment.is_empty() {
            return Err(VirtualPathError::EmptySegment);
        }
        if segment == "." || segment == ".." {
            return Err(VirtualPathError::DotSegment);
        }
    }
    Ok(())
}

#[cfg(test)]
mod path_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_empty_is_valid() {
        assert!(validate_virtual_path("").is_ok());
    }

    #[test]
    fn test_validate_simple_paths() {
        assert!(validate_virtual_path("notes").is_ok());
        assert!(validate_virtual_path("notes/rust").is_ok());
    }

    #[test]
    fn test_validate_leading_slash() {
        assert_eq!(validate_virtual_path("/notes"), Err(VirtualPathError::LeadingOrTrailingSlash));
    }

    #[test]
    fn test_validate_trailing_slash() {
        assert_eq!(validate_virtual_path("notes/"), Err(VirtualPathError::LeadingOrTrailingSlash));
    }

    #[test]
    fn test_validate_dot_segment() {
        assert_eq!(validate_virtual_path("notes/./rust"), Err(VirtualPathError::DotSegment));
        assert_eq!(validate_virtual_path("notes/../rust"), Err(VirtualPathError::DotSegment));
    }

    #[test]
    fn test_validate_double_slash() {
        assert_eq!(validate_virtual_path("notes//rust"), Err(VirtualPathError::EmptySegment));
    }

    #[test]
    fn test_validate_nul_byte() {
        assert_eq!(validate_virtual_path("notes/\0rust"), Err(VirtualPathError::NulByte));
    }

    #[test]
    fn test_find_vault_root_at_start() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".cerbo")).unwrap();
        assert_eq!(find_vault_root(temp.path()), Some(temp.path().to_path_buf()));
    }

    #[test]
    fn test_find_vault_root_in_ancestor() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".cerbo")).unwrap();
        let child = temp.path().join("a").join("b");
        std::fs::create_dir_all(&child).unwrap();
        assert_eq!(find_vault_root(&child), Some(temp.path().to_path_buf()));
    }

    #[test]
    fn test_find_vault_root_not_found() {
        let temp = TempDir::new().unwrap();
        assert!(find_vault_root(temp.path()).is_none());
    }

    fn make_ctx_with_vault(config_dir: &std::path::Path, vault_path: &std::path::Path) -> CerboContext {
        use crate::config::Config;
        let ctx = CerboContext {
            config_dir: config_dir.to_path_buf(),
            cache_dir: config_dir.to_path_buf(),
        };
        let cfg = Config {
            vaults: vec![crate::vault::Vault {
                id: "test-id".into(),
                name: "Test".into(),
                path: vault_path.to_path_buf(),
                is_auto: false,
            }],
        };
        let toml = toml::to_string_pretty(&cfg).unwrap();
        std::fs::create_dir_all(config_dir).unwrap();
        std::fs::write(config_dir.join("vaults.toml"), toml).unwrap();
        ctx
    }

    #[test]
    fn vault_id_from_path_match() {
        let tmp = TempDir::new().unwrap();
        let vault_dir = tmp.path().join("vault");
        std::fs::create_dir_all(&vault_dir).unwrap();
        let ctx = make_ctx_with_vault(&tmp.path().join("cfg"), &vault_dir);
        assert_eq!(vault_id_from_path(&ctx, &vault_dir), Some("test-id".into()));
    }

    #[test]
    fn vault_id_from_path_no_match() {
        let tmp = TempDir::new().unwrap();
        let vault_dir = tmp.path().join("vault");
        std::fs::create_dir_all(&vault_dir).unwrap();
        let ctx = make_ctx_with_vault(&tmp.path().join("cfg"), &vault_dir);
        let other = tmp.path().join("other");
        std::fs::create_dir_all(&other).unwrap();
        assert_eq!(vault_id_from_path(&ctx, &other), None);
    }

    #[test]
    fn vault_id_from_path_via_symlink() {
        let tmp = TempDir::new().unwrap();
        let vault_dir = tmp.path().join("real_vault");
        std::fs::create_dir_all(&vault_dir).unwrap();
        let link = tmp.path().join("link_vault");
        std::os::unix::fs::symlink(&vault_dir, &link).unwrap();
        let ctx = make_ctx_with_vault(&tmp.path().join("cfg"), &vault_dir);
        // accessing through symlink should still resolve to the same vault
        assert_eq!(vault_id_from_path(&ctx, &link), Some("test-id".into()));
    }
}
