//! What an agent may do in a vault (design D6, D8).
//!
//! The level of each vault is stored by vault id in `agents.toml` beside
//! `vaults.toml`; a vault with no entry is `read`. The daemon reads the file on
//! every agent write, so a change applies to connected agents at once. Every
//! applied agent change is appended to `agent-changes.jsonl` in the same
//! directory, the audit trail for `auto`.

use crate::CerboContext;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

/// How long an approval request waits for an answer when nothing is set.
pub const DEFAULT_APPROVAL_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// The agent permission of one vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentPermission {
    /// The agent can only read. The default.
    #[default]
    Read,
    /// The agent's changes apply at once (`manage-auto`).
    Auto,
    /// Each change waits for a person (`manage-approve`).
    Approve,
}

impl AgentPermission {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentPermission::Read => "read",
            AgentPermission::Auto => "auto",
            AgentPermission::Approve => "approve",
        }
    }

    /// Whether the agent may ask to change the vault at all.
    pub fn can_write(self) -> bool {
        self != AgentPermission::Read
    }
}

impl std::fmt::Display for AgentPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AgentPermission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "read" => Ok(AgentPermission::Read),
            "auto" => Ok(AgentPermission::Auto),
            "approve" => Ok(AgentPermission::Approve),
            other => Err(format!("unknown agent permission {other:?}: use read, auto or approve")),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct AgentSettings {
    /// Seconds an approval request waits before it counts as rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    approval_timeout_secs: Option<u64>,
    /// Vault id to level.
    #[serde(default)]
    permissions: BTreeMap<String, AgentPermission>,
}

fn settings_path(ctx: &CerboContext) -> PathBuf {
    ctx.config_dir.join("agents.toml")
}

fn load(ctx: &CerboContext) -> Result<AgentSettings, String> {
    let path = settings_path(ctx);
    match std::fs::read_to_string(&path) {
        Ok(raw) => toml::from_str(&raw).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AgentSettings::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// The level of `vault_id`; `read` when never set. A file that cannot be read
/// also gives `read`: a broken setting must never widen access.
pub fn get_permission(ctx: &CerboContext, vault_id: &str) -> AgentPermission {
    load(ctx).ok().and_then(|s| s.permissions.get(vault_id).copied()).unwrap_or_default()
}

/// Store the level of `vault_id`.
pub fn set_permission(ctx: &CerboContext, vault_id: &str, level: AgentPermission) -> Result<(), String> {
    let mut settings = load(ctx)?;
    settings.permissions.insert(vault_id.to_string(), level);
    let raw = toml::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&ctx.config_dir).map_err(|e| format!("{}: {e}", ctx.config_dir.display()))?;
    crate::fsio::write_atomic_str(&settings_path(ctx), &raw).map_err(|e| format!("agents.toml: {e}"))
}

/// How long an approval request waits: `approval_timeout_secs` of
/// `agents.toml`, else [`DEFAULT_APPROVAL_TIMEOUT`].
pub fn approval_timeout(ctx: &CerboContext) -> Duration {
    load(ctx)
        .ok()
        .and_then(|s| s.approval_timeout_secs)
        .filter(|secs| *secs > 0)
        .map_or(DEFAULT_APPROVAL_TIMEOUT, Duration::from_secs)
}

/// One applied agent change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeEntry {
    /// Unix seconds.
    pub time: u64,
    /// Vault id.
    pub vault: String,
    pub page: String,
    /// The agent's name, as it told the daemon.
    pub agent: String,
    /// `create`, `edit`, `rename`, `move`, `trash` or `link`.
    pub action: String,
    /// Whether a person approved it (`approve`) or it applied by itself (`auto`).
    pub approved: bool,
}

fn log_path(ctx: &CerboContext) -> PathBuf {
    ctx.config_dir.join("agent-changes.jsonl")
}

/// Append `entry` to the change log.
pub fn append_change(ctx: &CerboContext, entry: &ChangeEntry) -> Result<(), String> {
    let mut line = serde_json::to_string(entry).map_err(|e| e.to_string())?;
    line.push('\n');
    std::fs::create_dir_all(&ctx.config_dir).map_err(|e| e.to_string())?;
    let path = log_path(ctx);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.write_all(line.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))
}

/// The last `limit` changes of `vault_id`, oldest first.
pub fn read_changes(ctx: &CerboContext, vault_id: &str, limit: usize) -> Result<Vec<ChangeEntry>, String> {
    let path = log_path(ctx);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let mut entries: Vec<ChangeEntry> = raw
        .lines()
        .filter_map(|line| serde_json::from_str::<ChangeEntry>(line).ok())
        .filter(|e| e.vault == vault_id)
        .collect();
    let skip = entries.len().saturating_sub(limit);
    entries.drain(..skip);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dir: &std::path::Path) -> CerboContext {
        CerboContext { config_dir: dir.join("config"), cache_dir: dir.join("cache") }
    }

    #[test]
    fn a_vault_never_configured_is_read_only() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(get_permission(&ctx(tmp.path()), "v1"), AgentPermission::Read);
    }

    #[test]
    fn levels_are_stored_per_vault() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = ctx(tmp.path());
        set_permission(&ctx, "a", AgentPermission::Auto).unwrap();
        set_permission(&ctx, "b", AgentPermission::Approve).unwrap();
        assert_eq!(get_permission(&ctx, "a"), AgentPermission::Auto);
        assert_eq!(get_permission(&ctx, "b"), AgentPermission::Approve);
        assert_eq!(get_permission(&ctx, "c"), AgentPermission::Read);
        set_permission(&ctx, "a", AgentPermission::Read).unwrap();
        assert_eq!(get_permission(&ctx, "a"), AgentPermission::Read);
    }

    #[test]
    fn a_broken_file_never_widens_access() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = ctx(tmp.path());
        std::fs::create_dir_all(&ctx.config_dir).unwrap();
        std::fs::write(ctx.config_dir.join("agents.toml"), "permissions = [[[").unwrap();
        assert_eq!(get_permission(&ctx, "a"), AgentPermission::Read);
    }

    #[test]
    fn parses_the_three_words_and_nothing_else() {
        assert_eq!("auto".parse::<AgentPermission>(), Ok(AgentPermission::Auto));
        assert_eq!("approve".parse::<AgentPermission>(), Ok(AgentPermission::Approve));
        assert_eq!("read".parse::<AgentPermission>(), Ok(AgentPermission::Read));
        assert!("write".parse::<AgentPermission>().is_err());
    }

    #[test]
    fn the_approval_timeout_defaults_to_ten_minutes_and_can_be_set() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = ctx(tmp.path());
        assert_eq!(approval_timeout(&ctx), Duration::from_secs(600));
        std::fs::create_dir_all(&ctx.config_dir).unwrap();
        std::fs::write(ctx.config_dir.join("agents.toml"), "approval_timeout_secs = 137\n").unwrap();
        assert_eq!(approval_timeout(&ctx), Duration::from_secs(137));
    }

    #[test]
    fn the_change_log_keeps_the_last_entries_of_one_vault() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = ctx(tmp.path());
        for n in 0..5u64 {
            let vault = if n % 2 == 0 { "a" } else { "b" };
            let entry = ChangeEntry {
                time: n,
                vault: vault.into(),
                page: format!("p{n}"),
                agent: "opencode".into(),
                action: "edit".into(),
                approved: false,
            };
            append_change(&ctx, &entry).unwrap();
        }
        let a = read_changes(&ctx, "a", 2).unwrap();
        assert_eq!(a.iter().map(|e| e.page.as_str()).collect::<Vec<_>>(), ["p2", "p4"]);
        assert!(read_changes(&ctx, "none", 10).unwrap().is_empty());
    }
}
