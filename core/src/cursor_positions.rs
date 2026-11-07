//! Where the last editing session on each page ended (spec
//! `editor-position-restore`): caret line and column plus the write view's
//! scroll offset, per vault id and page uuid. Kept in `cursor.toml` in the
//! config dir next to `ui.toml`, because it is per-machine UI state and never
//! belongs in the vault.

use crate::paths;
use crate::CerboContext;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// Most positions kept across all vaults; the least recently saved go first.
pub const MAX_ENTRIES: usize = 10_000;

/// Saves are read-modify-write of one file and may run on several threads.
static FILE_LOCK: Mutex<()> = Mutex::new(());

/// What the editor stores and restores. `line` and `column` are 1-based.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
    #[serde(default)]
    pub scroll_top: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredPosition {
    #[serde(flatten)]
    position: CursorPosition,
    /// Unix milliseconds of the last save; orders entries for pruning.
    #[serde(default)]
    saved_at: u64,
}

/// `cursor.toml`: vault id → page uuid → position.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CursorFile {
    #[serde(default)]
    vaults: BTreeMap<String, BTreeMap<String, StoredPosition>>,
}

fn cursor_path(ctx: &CerboContext) -> Result<PathBuf, String> {
    Ok(paths::config_dir(ctx.config_dir.clone())?.join("cursor.toml"))
}

fn read_file(ctx: &CerboContext) -> Result<CursorFile, String> {
    let p = cursor_path(ctx)?;
    if !p.exists() {
        return Ok(CursorFile::default());
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| format!("cursor_positions read: {e}"))?;
    toml::from_str(&raw).map_err(|e| format!("cursor_positions parse: {e}"))
}

fn write_file(ctx: &CerboContext, file: &CursorFile) -> Result<(), String> {
    let p = cursor_path(ctx)?;
    let raw = toml::to_string(file).map_err(|e| format!("cursor_positions serialize: {e}"))?;
    crate::fsio::write_atomic_str(&p, &raw).map_err(|e| format!("cursor_positions write: {e}"))
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The stored position of `uuid` in `vault_id`, if any. A missing or
/// unreadable `cursor.toml` means nothing is stored.
pub fn load_cursor_position(ctx: &CerboContext, vault_id: &str, uuid: &str) -> Result<Option<CursorPosition>, String> {
    let _guard = FILE_LOCK.lock().map_err(|_| "cursor_positions lock poisoned".to_string())?;
    let file = read_file(ctx).unwrap_or_default();
    Ok(file.vaults.get(vault_id).and_then(|pages| pages.get(uuid)).map(|s| s.position))
}

/// Remember where editing `uuid` in `vault_id` ended. When `uuid` gets its
/// first entry, entries of this vault whose page no longer exists
/// (`page_exists` false) are dropped; later saves of the same page, which
/// run 1 s after every edit, skip that check. The file is capped at
/// [`MAX_ENTRIES`] by dropping the least recently saved. Saving the position
/// already stored writes nothing.
pub fn save_cursor_position(
    ctx: &CerboContext,
    vault_id: &str,
    uuid: &str,
    position: CursorPosition,
    page_exists: impl Fn(&str) -> bool,
) -> Result<(), String> {
    save_at(ctx, vault_id, uuid, position, now_millis(), page_exists)
}

fn save_at(
    ctx: &CerboContext,
    vault_id: &str,
    uuid: &str,
    position: CursorPosition,
    saved_at: u64,
    page_exists: impl Fn(&str) -> bool,
) -> Result<(), String> {
    let _guard = FILE_LOCK.lock().map_err(|_| "cursor_positions lock poisoned".to_string())?;
    // A broken file is UI state only: start over rather than block saving.
    let mut file = read_file(ctx).unwrap_or_default();
    if file.vaults.get(vault_id).and_then(|pages| pages.get(uuid)).map(|s| s.position) == Some(position) {
        return Ok(());
    }

    let pages = file.vaults.entry(vault_id.to_string()).or_default();
    if !pages.contains_key(uuid) {
        pages.retain(|page, _| page_exists(page));
    }
    pages.insert(uuid.to_string(), StoredPosition { position, saved_at });
    prune(&mut file, MAX_ENTRIES);
    write_file(ctx, &file)
}

/// Drop the least recently saved entries until at most `max` remain, and any
/// vault table left empty.
fn prune(file: &mut CursorFile, max: usize) {
    let total: usize = file.vaults.values().map(BTreeMap::len).sum();
    if total > max {
        let mut stamps: Vec<u64> = file.vaults.values().flat_map(|pages| pages.values().map(|s| s.saved_at)).collect();
        stamps.sort_unstable();
        // Keep entries newer than the cut-off; ties at the cut-off are kept
        // only while there is room.
        let cutoff = stamps[total - max];
        let mut room_at_cutoff = stamps[total - max..].iter().filter(|&&s| s == cutoff).count();
        for pages in file.vaults.values_mut() {
            pages.retain(|_, s| {
                if s.saved_at > cutoff {
                    true
                } else if s.saved_at == cutoff && room_at_cutoff > 0 {
                    room_at_cutoff -= 1;
                    true
                } else {
                    false
                }
            });
        }
    }
    file.vaults.retain(|_, pages| !pages.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ctx(tmp: &TempDir) -> CerboContext {
        CerboContext { config_dir: tmp.path().join("config"), cache_dir: tmp.path().join("cache") }
    }

    fn pos(line: u32, column: u32, scroll_top: f64) -> CursorPosition {
        CursorPosition { line, column, scroll_top }
    }

    #[test]
    fn nothing_stored_without_cursor_toml() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(load_cursor_position(&ctx(&tmp), "vault-a", "page-1").unwrap(), None);
    }

    #[test]
    fn save_and_load_round_trip_per_vault_and_page() {
        let tmp = TempDir::new().unwrap();
        let ctx = ctx(&tmp);
        save_cursor_position(&ctx, "vault-a", "page-1", pos(137, 4, 1370.5), |_| true).unwrap();
        save_cursor_position(&ctx, "vault-b", "page-1", pos(2, 1, 0.0), |_| true).unwrap();

        assert!(ctx.config_dir.join("cursor.toml").exists());
        assert_eq!(load_cursor_position(&ctx, "vault-a", "page-1").unwrap(), Some(pos(137, 4, 1370.5)));
        assert_eq!(load_cursor_position(&ctx, "vault-b", "page-1").unwrap(), Some(pos(2, 1, 0.0)));
        assert_eq!(load_cursor_position(&ctx, "vault-a", "page-2").unwrap(), None);

        // A later save replaces the earlier one.
        save_cursor_position(&ctx, "vault-a", "page-1", pos(3, 7, 0.0), |_| true).unwrap();
        assert_eq!(load_cursor_position(&ctx, "vault-a", "page-1").unwrap(), Some(pos(3, 7, 0.0)));
    }

    #[test]
    fn saving_drops_missing_pages_of_the_same_vault_only() {
        let tmp = TempDir::new().unwrap();
        let ctx = ctx(&tmp);
        save_cursor_position(&ctx, "vault-a", "gone", pos(5, 1, 0.0), |_| true).unwrap();
        save_cursor_position(&ctx, "vault-b", "gone", pos(5, 1, 0.0), |_| true).unwrap();

        save_cursor_position(&ctx, "vault-a", "page-1", pos(137, 1, 0.0), |p| p != "gone").unwrap();

        assert_eq!(load_cursor_position(&ctx, "vault-a", "gone").unwrap(), None);
        assert_eq!(load_cursor_position(&ctx, "vault-b", "gone").unwrap(), Some(pos(5, 1, 0.0)));
        assert_eq!(load_cursor_position(&ctx, "vault-a", "page-1").unwrap(), Some(pos(137, 1, 0.0)));
    }

    #[test]
    fn saving_a_known_page_again_checks_no_other_page() {
        let tmp = TempDir::new().unwrap();
        let ctx = ctx(&tmp);
        save_cursor_position(&ctx, "vault-a", "page-1", pos(1, 1, 0.0), |_| true).unwrap();
        save_cursor_position(&ctx, "vault-a", "page-2", pos(1, 1, 0.0), |_| true).unwrap();

        save_cursor_position(&ctx, "vault-a", "page-1", pos(137, 1, 0.0), |p| panic!("checked {p}")).unwrap();

        assert_eq!(load_cursor_position(&ctx, "vault-a", "page-2").unwrap(), Some(pos(1, 1, 0.0)));
        assert_eq!(load_cursor_position(&ctx, "vault-a", "page-1").unwrap(), Some(pos(137, 1, 0.0)));
    }

    #[test]
    fn saving_the_same_position_writes_nothing() {
        let tmp = TempDir::new().unwrap();
        let ctx = ctx(&tmp);
        save_at(&ctx, "vault-a", "page-1", pos(137, 1, 0.0), 1, |_| true).unwrap();
        let before = std::fs::read_to_string(ctx.config_dir.join("cursor.toml")).unwrap();
        save_at(&ctx, "vault-a", "page-1", pos(137, 1, 0.0), 2, |_| true).unwrap();
        assert_eq!(std::fs::read_to_string(ctx.config_dir.join("cursor.toml")).unwrap(), before);
    }

    #[test]
    fn prune_keeps_the_most_recent_entries() {
        let mut file = CursorFile::default();
        for i in 0..137u64 {
            let vault = if i % 2 == 0 { "vault-a" } else { "vault-b" };
            file.vaults
                .entry(vault.into())
                .or_default()
                .insert(format!("page-{i}"), StoredPosition { position: pos(1, 1, 0.0), saved_at: i });
        }
        prune(&mut file, 10);

        let mut kept: Vec<u64> = file.vaults.values().flat_map(|p| p.values().map(|s| s.saved_at)).collect();
        kept.sort_unstable();
        assert_eq!(kept, (127..137).collect::<Vec<_>>());
    }

    #[test]
    fn prune_with_equal_timestamps_keeps_exactly_max_and_drops_empty_vaults() {
        let mut file = CursorFile::default();
        file.vaults.entry("old".into()).or_default().insert("p".into(), StoredPosition { position: pos(1, 1, 0.0), saved_at: 1 });
        for i in 0..5 {
            file.vaults
                .entry("new".into())
                .or_default()
                .insert(format!("page-{i}"), StoredPosition { position: pos(1, 1, 0.0), saved_at: 7 });
        }
        prune(&mut file, 3);
        assert_eq!(file.vaults.values().map(BTreeMap::len).sum::<usize>(), 3);
        assert!(!file.vaults.contains_key("old"));
    }

    #[test]
    fn save_caps_the_file_at_max_entries() {
        let tmp = TempDir::new().unwrap();
        let ctx = ctx(&tmp);
        let mut file = CursorFile::default();
        let pages = file.vaults.entry("vault-a".into()).or_default();
        for i in 0..MAX_ENTRIES as u64 {
            pages.insert(format!("page-{i}"), StoredPosition { position: pos(1, 1, 0.0), saved_at: i + 1 });
        }
        write_file(&ctx, &file).unwrap();

        save_at(&ctx, "vault-b", "new", pos(137, 1, 0.0), u64::MAX, |_| true).unwrap();

        let file = read_file(&ctx).unwrap();
        assert_eq!(file.vaults.values().map(BTreeMap::len).sum::<usize>(), MAX_ENTRIES);
        assert!(!file.vaults["vault-a"].contains_key("page-0"), "oldest entry dropped");
        assert_eq!(load_cursor_position(&ctx, "vault-b", "new").unwrap(), Some(pos(137, 1, 0.0)));
    }

    #[test]
    fn broken_cursor_toml_reads_as_empty_and_is_replaced_on_save() {
        let tmp = TempDir::new().unwrap();
        let ctx = ctx(&tmp);
        std::fs::create_dir_all(&ctx.config_dir).unwrap();
        crate::fsio::write_atomic_str(&ctx.config_dir.join("cursor.toml"), "not = [toml").unwrap();

        assert_eq!(load_cursor_position(&ctx, "vault-a", "page-1").unwrap(), None);
        save_cursor_position(&ctx, "vault-a", "page-1", pos(2, 1, 0.0), |_| true).unwrap();
        assert_eq!(load_cursor_position(&ctx, "vault-a", "page-1").unwrap(), Some(pos(2, 1, 0.0)));
    }
}
