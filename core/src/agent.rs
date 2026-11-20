//! What an agent may do in a vault (design D6, D8).
//!
//! The level of each vault is stored by vault id in `agents.toml` beside
//! `vaults.toml`; a vault with no entry is `read`. The daemon reads the file on
//! every agent write, so a change applies to connected agents at once. Every
//! applied agent change is appended to `agent-changes.jsonl` in the same
//! directory, the audit trail for `auto`.

use crate::host_access::{Guard, HostPath, HostSettings};
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
    /// Vault id to host access (see [`crate::host_access`]).
    #[serde(default)]
    host: BTreeMap<String, HostSettings>,
    /// The `[agent]` table: how the daemon runs agent CLIs.
    #[serde(default)]
    agent: AgentSection,
}

/// The `[agent]` table of `agents.toml`: how the daemon runs agent CLIs.
///
/// ```toml
/// [agent]
/// sandbox = "bwrap"            # or "none": run agents unsandboxed, with a warning
/// opencode = "/abs/opencode"   # optional, else found on PATH at daemon start
/// claude = "/abs/claude-agent-acp"
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSection {
    /// `bwrap` (default) or `none`. Kept as text so a typo costs the agent
    /// sessions an error message, not the whole file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,
    /// Absolute path of the OpenCode binary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode: Option<String>,
    /// Absolute path of the `claude-agent-acp` adapter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude: Option<String>,
}

/// How agent processes are confined (design D8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SandboxMode {
    #[default]
    Bwrap,
    None,
}

impl AgentSection {
    /// The sandbox mode; an unknown value is an error, never a silent `none`.
    pub fn sandbox_mode(&self) -> Result<SandboxMode, String> {
        match self.sandbox.as_deref() {
            None | Some("bwrap") => Ok(SandboxMode::Bwrap),
            Some("none") => Ok(SandboxMode::None),
            Some(other) => Err(format!("agents.toml: [agent] sandbox = {other:?} is not valid: use \"bwrap\" or \"none\"")),
        }
    }
}

/// The `[agent]` table of `agents.toml`; the defaults when the file is absent
/// or cannot be read (which keeps the sandbox on).
pub fn agent_section(ctx: &CerboContext) -> AgentSection {
    load(ctx).map(|s| s.agent).unwrap_or_default()
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
    save(ctx, &settings)
}

fn save(ctx: &CerboContext, settings: &AgentSettings) -> Result<(), String> {
    let raw = toml::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&ctx.config_dir).map_err(|e| format!("{}: {e}", ctx.config_dir.display()))?;
    crate::fsio::write_atomic_str(&settings_path(ctx), &raw).map_err(|e| format!("agents.toml: {e}"))
}

/// The host access of `vault_id`: off, with no paths, when never set. A file
/// that cannot be read also gives off: a broken setting must never widen access.
pub fn get_host(ctx: &CerboContext, vault_id: &str) -> HostSettings {
    load(ctx).ok().and_then(|s| s.host.get(vault_id).cloned()).unwrap_or_default()
}

/// Turn host access of `vault_id` on or off; the list is kept.
pub fn set_host_enabled(ctx: &CerboContext, vault_id: &str, enabled: bool) -> Result<(), String> {
    let mut settings = load(ctx)?;
    settings.host.entry(vault_id.to_string()).or_default().enabled = enabled;
    save(ctx, &settings)
}

/// Allow `path` (already checked by [`crate::host_access::accept_entry`]); an
/// entry for the same path is replaced, so `write` can be raised or lowered.
pub fn add_host_path(ctx: &CerboContext, vault_id: &str, path: PathBuf, write: bool) -> Result<(), String> {
    let mut settings = load(ctx)?;
    let host = settings.host.entry(vault_id.to_string()).or_default();
    host.paths.retain(|p| p.path != path);
    host.paths.push(HostPath { path, write });
    host.paths.sort_by(|a, b| a.path.cmp(&b.path));
    save(ctx, &settings)
}

/// Stop allowing `path`; false when it was not on the list.
pub fn remove_host_path(ctx: &CerboContext, vault_id: &str, path: &std::path::Path) -> Result<bool, String> {
    let mut settings = load(ctx)?;
    let host = settings.host.entry(vault_id.to_string()).or_default();
    let before = host.paths.len();
    host.paths.retain(|p| p.path != path);
    let removed = host.paths.len() != before;
    save(ctx, &settings)?;
    Ok(removed)
}

/// What decides which places are never reachable on this machine, other than
/// the daemon's own directories: the other vaults' roots and the home folder.
pub fn host_guard(vault_root: PathBuf, other_vaults: Vec<PathBuf>, protected: Vec<PathBuf>) -> Guard {
    Guard {
        vault_root,
        other_vaults,
        protected,
        home: std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from),
    }
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
    /// How it ended, for host actions: `done`, `rejected`, `timed_out` or
    /// `refused`. Vault changes are logged only when applied and leave it out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
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
#[allow(clippy::disallowed_methods)] // the tests write fixture files directly
mod tests {
    use super::*;

    fn ctx(dir: &std::path::Path) -> CerboContext {
        CerboContext { config_dir: dir.join("config"), cache_dir: dir.join("cache") }
    }

    #[test]
    fn host_access_is_off_until_set_and_keeps_its_list() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = ctx(tmp.path());
        assert!(!get_host(&ctx, "v").enabled);
        add_host_path(&ctx, "v", "/data/in".into(), false).unwrap();
        add_host_path(&ctx, "v", "/data/out".into(), true).unwrap();
        set_host_enabled(&ctx, "v", true).unwrap();
        set_host_enabled(&ctx, "v", false).unwrap();
        let host = get_host(&ctx, "v");
        assert!(!host.enabled);
        assert_eq!(host.paths.len(), 2);
        assert!(!get_host(&ctx, "other").enabled);
    }

    #[test]
    fn adding_a_path_again_changes_its_mode_and_removing_forgets_it() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = ctx(tmp.path());
        add_host_path(&ctx, "v", "/data/in".into(), false).unwrap();
        add_host_path(&ctx, "v", "/data/in".into(), true).unwrap();
        assert_eq!(get_host(&ctx, "v").paths, [HostPath { path: "/data/in".into(), write: true }]);
        assert!(remove_host_path(&ctx, "v", std::path::Path::new("/data/in")).unwrap());
        assert!(!remove_host_path(&ctx, "v", std::path::Path::new("/data/in")).unwrap());
    }

    #[test]
    fn host_settings_and_permission_do_not_disturb_each_other() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = ctx(tmp.path());
        set_permission(&ctx, "v", AgentPermission::Auto).unwrap();
        set_host_enabled(&ctx, "v", true).unwrap();
        assert_eq!(get_permission(&ctx, "v"), AgentPermission::Auto);
        set_permission(&ctx, "v", AgentPermission::Read).unwrap();
        assert!(get_host(&ctx, "v").enabled);
    }

    #[test]
    fn the_agent_section_defaults_to_the_sandbox_and_survives_a_permission_change() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = crate::CerboContext { config_dir: tmp.path().into(), cache_dir: tmp.path().into() };
        assert_eq!(agent_section(&ctx).sandbox_mode(), Ok(SandboxMode::Bwrap));
        std::fs::write(
            ctx.config_dir.join("agents.toml"),
            "[agent]\nsandbox = \"none\"\nopencode = \"/opt/oc\"\n",
        )
        .unwrap();
        set_permission(&ctx, "v137", AgentPermission::Auto).unwrap();
        let section = agent_section(&ctx);
        assert_eq!(section.sandbox_mode(), Ok(SandboxMode::None));
        assert_eq!(section.opencode.as_deref(), Some("/opt/oc"));
        let typo = AgentSection { sandbox: Some("bwrapp".into()), ..Default::default() };
        assert!(typo.sandbox_mode().unwrap_err().contains("bwrapp"));
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
                outcome: None,
            };
            append_change(&ctx, &entry).unwrap();
        }
        let a = read_changes(&ctx, "a", 2).unwrap();
        assert_eq!(a.iter().map(|e| e.page.as_str()).collect::<Vec<_>>(), ["p2", "p4"]);
        assert!(read_changes(&ctx, "none", 10).unwrap().is_empty());
    }
}
