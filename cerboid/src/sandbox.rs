//! The agent sandbox (design D8, D10).
//!
//! The daemon never starts an agent directly. [`plan`] turns what is known
//! about a session (agent binary, vault, socket directory, agent home) into a
//! [`Launch`]: either `bwrap` with a fixed argument list, or, only when
//! `agents.toml` says `[agent] sandbox = "none"`, the agent itself with a
//! warning. Building the argument list is pure ([`bwrap_args`]); the host is
//! looked at through the [`HostFs`] trait so tests can show exactly what is
//! bound on a NixOS host and on a plain one.
//!
//! What the agent sees inside: the host network (it needs its model API),
//! `/nix`, `/etc` (and `/.host-etc` on NixOS), `/run/current-system`, `/usr`,
//! `/bin` and `/lib*` where they exist, all read-only; a private `/tmp`,
//! `/dev`, `/proc`; empty tmpfs over `/home` and `/run`; the vault read-only
//! at its real path and as the working directory; its own home read-write at
//! [`SANDBOX_HOME`]; one directory with the session socket at
//! [`SANDBOX_SOCKET_DIR`]; and, as single-file binds, only its login file.
//! Nothing else of the user's home and never the daemon's main socket.

use crate::agents::{self, HostEnv};
use cerbo_core::rpc::AgentKind;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The agent's home inside the sandbox.
pub const SANDBOX_HOME: &str = "/home/agent";
/// Where the session's socket directory appears inside the sandbox.
pub const SANDBOX_SOCKET_DIR: &str = "/run/cerbo/session";
/// The socket file inside [`SANDBOX_SOCKET_DIR`].
pub const SESSION_SOCKET_NAME: &str = "agent.sock";

/// How long a failed probe is remembered before bwrap is tried again.
const FAILED_PROBE_TTL: Duration = Duration::from_secs(10);

// ── Host view ────────────────────────────────────────────────────────────────

/// What the argument builder needs to know about the host's files.
pub trait HostFs {
    fn exists(&self, path: &Path) -> bool;
    /// The target of `path` when it is a symbolic link.
    fn symlink_target(&self, path: &Path) -> Option<PathBuf>;
}

/// The real file system.
pub struct RealFs;

impl HostFs for RealFs {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
    fn symlink_target(&self, path: &Path) -> Option<PathBuf> {
        std::fs::read_link(path).ok()
    }
}

/// The read-only parts of the host the agent's programs live in. NixOS has
/// `/nix`, `/etc`, `/.host-etc` (where `/etc/resolv.conf` and the certificates
/// point) and `/run/current-system`; other systems `/usr`, `/bin`, `/lib*`.
/// Each is bound only where it exists.
const SYSTEM_PATHS: [&str; 10] =
    ["/nix", "/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/etc", "/.host-etc", "/run/current-system"];

// ── The plan ─────────────────────────────────────────────────────────────────

/// What to run for one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    /// The whole environment of the process: nothing else is inherited.
    pub env: Vec<(OsString, OsString)>,
    /// Working directory for the process itself (`None` for bwrap, which has `--chdir`).
    pub cwd: Option<PathBuf>,
    pub sandboxed: bool,
    /// Shown on every session that is not sandboxed.
    pub warning: Option<String>,
}

impl Launch {
    /// A command with exactly this environment and working directory.
    pub fn command(&self) -> std::process::Command {
        let mut command = std::process::Command::new(&self.program);
        command.args(&self.args).env_clear().envs(self.env.iter().map(|(k, v)| (k, v)));
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }
        command
    }
}

/// Everything [`plan`] needs for one session.
#[derive(Debug, Clone)]
pub struct SessionSpec {
    pub kind: AgentKind,
    /// Real path of the agent binary (from the registry).
    pub agent_binary: PathBuf,
    /// Real path of the vault, absolute.
    pub vault: PathBuf,
    /// Host directory of this session's socket (holds `agent.sock`).
    pub socket_dir: PathBuf,
    /// Host path of the agent's home (see [`prepare_home`]).
    pub agent_home: PathBuf,
    /// Real path of the `cerbo` binary, which the agent runs as `cerbo mcp`.
    pub cerbo_bin: PathBuf,
    /// The agent's login file on the real home, when it exists.
    pub login_file: Option<PathBuf>,
    /// The home directory of the real user: never bound, never an extra root.
    pub real_home: Option<PathBuf>,
    /// The host's `PATH`, to find an interpreter named by a `#!/usr/bin/env` line.
    pub host_path: Option<OsString>,
}

/// How the agent is confined: a path to `bwrap`, or not at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confinement {
    Bwrap(PathBuf),
    None,
}

/// Decide the confinement from `[agent] sandbox` and `CERBO_BWRAP`. `bwrap`
/// mode needs an absolute `CERBO_BWRAP` that passes [`probe`]; anything else
/// is an error that says why, never a silent fallback.
pub fn confinement(mode: cerbo_core::agent::SandboxMode, bwrap_env: Option<OsString>) -> Result<Confinement, String> {
    use cerbo_core::agent::SandboxMode;
    if mode == SandboxMode::None {
        return Ok(Confinement::None);
    }
    let path = bwrap_env.filter(|v| !v.is_empty()).map(PathBuf::from).ok_or(
        "agent sandbox unavailable: CERBO_BWRAP is not set. The Nix package of cerboid sets it; \
         otherwise set CERBO_BWRAP to the absolute path of bubblewrap, or accept running agents \
         without a sandbox with `[agent] sandbox = \"none\"` in agents.toml",
    )?;
    if !path.is_absolute() {
        return Err(format!("agent sandbox unavailable: CERBO_BWRAP must be an absolute path, not {}", path.display()));
    }
    probe(&path)?;
    Ok(Confinement::Bwrap(path))
}

/// Build the launch for a session.
pub fn plan(confinement: &Confinement, spec: &SessionSpec, fs: &dyn HostFs) -> Result<Launch, String> {
    let (_, agent_args) = agents::command_of(spec.kind);
    match confinement {
        Confinement::Bwrap(bwrap) => {
            let args = bwrap_args(spec, fs)?;
            Ok(Launch { program: bwrap.clone(), args: args.into_iter().map(OsString::from).collect(), env: Vec::new(), cwd: None, sandboxed: true, warning: None })
        }
        Confinement::None => {
            let args: Vec<OsString> = agent_args.iter().map(OsString::from).collect();
            let socket = spec.socket_dir.join(SESSION_SOCKET_NAME);
            link_login(spec)?;
            let mut vars = agent_env(spec.kind, &spec.agent_home, &socket, &spec.agent_binary, &spec.cerbo_bin, spec.host_path.as_ref(), &[]);
            vars.sort();
            Ok(Launch {
                program: spec.agent_binary.clone(),
                args,
                env: vars.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
                cwd: Some(spec.vault.clone()),
                sandboxed: false,
                warning: Some(
                    "The agent runs without a sandbox (agents.toml: [agent] sandbox = \"none\"): it can read and write every file you can."
                        .to_string(),
                ),
            })
        }
    }
}

// ── bwrap argument list ──────────────────────────────────────────────────────

/// The `bwrap` arguments for a session, up to and including the agent command.
pub fn bwrap_args(spec: &SessionSpec, fs: &dyn HostFs) -> Result<Vec<String>, String> {
    let mut a: Vec<String> = Vec::new();
    let mut push = |items: &[&str]| a.extend(items.iter().map(|s| s.to_string()));

    // `--share-net` must come after `--unshare-all`.
    push(&["--unshare-all", "--share-net", "--die-with-parent", "--new-session", "--clearenv"]);
    push(&["--dev", "/dev", "--proc", "/proc", "--tmpfs", "/tmp", "--tmpfs", "/home", "--tmpfs", "/run"]);

    let mut bound: Vec<PathBuf> = Vec::new();
    for path in SYSTEM_PATHS {
        let path = Path::new(path);
        if let Some(target) = fs.symlink_target(path) {
            // usrmerge: `/bin -> usr/bin`. Keep it a link; `/usr` is bound.
            if !target.is_absolute() && path != Path::new("/run/current-system") {
                push(&["--symlink", &target.to_string_lossy(), &path.to_string_lossy()]);
                bound.push(path.to_path_buf());
                continue;
            }
        }
        if fs.exists(path) {
            push(&["--ro-bind", &path.to_string_lossy(), &path.to_string_lossy()]);
            bound.push(path.to_path_buf());
        }
    }

    // The agent and what it needs to start, when they lie outside the system paths.
    let mut roots = runtime_roots(&spec.agent_binary, spec.host_path.as_ref(), fs);
    roots.extend(runtime_roots(&spec.cerbo_bin, spec.host_path.as_ref(), fs));
    let mut path_dirs: Vec<PathBuf> = Vec::new();
    for (root, bin_dir) in roots {
        if let Some(dir) = bin_dir {
            if !path_dirs.contains(&dir) {
                path_dirs.push(dir);
            }
        }
        if let Some(root) = root {
            check_root(&root, spec)?;
            if !bound.iter().any(|b| root.starts_with(b)) && !bound.contains(&root) {
                push(&["--ro-bind", &root.to_string_lossy(), &root.to_string_lossy()]);
                bound.push(root);
            }
        }
    }

    // The vault: read-only at its real path, so paths in the agent's view
    // match the daemon's.
    let vault = spec.vault.to_string_lossy().to_string();
    if !spec.vault.is_absolute() {
        return Err(format!("the vault path {vault} is not absolute"));
    }
    push(&["--ro-bind", &vault, &vault]);
    // The agent's own home, writable, with the login file bound over it.
    push(&["--bind", &spec.agent_home.to_string_lossy(), SANDBOX_HOME]);
    if let Some(login) = &spec.login_file {
        let inside = login_inside(spec.kind, SANDBOX_HOME);
        push(&["--bind", &login.to_string_lossy(), &inside.to_string_lossy()]);
    }
    // The session's socket directory, and nothing of the daemon's own.
    push(&["--bind", &spec.socket_dir.to_string_lossy(), SANDBOX_SOCKET_DIR]);
    push(&["--chdir", &vault]);

    // Environment: only what the agent needs.
    let socket = Path::new(SANDBOX_SOCKET_DIR).join(SESSION_SOCKET_NAME);
    let vars = agent_env(spec.kind, Path::new(SANDBOX_HOME), &socket, &spec.agent_binary, &spec.cerbo_bin, None, &path_dirs);
    for (k, v) in vars {
        push(&["--setenv", &k, &v]);
    }

    let (_, agent_args) = agents::command_of(spec.kind);
    push(&["--"]);
    a.push(spec.agent_binary.to_string_lossy().to_string());
    a.extend(agent_args.iter().map(|s| s.to_string()));
    Ok(a)
}

/// Where the login file lives in a home rooted at `home`.
pub fn login_inside(kind: AgentKind, home: &str) -> PathBuf {
    match kind {
        AgentKind::Opencode => Path::new(home).join(".local/share/opencode/auth.json"),
        AgentKind::Claude => Path::new(home).join(".claude/.credentials.json"),
    }
}

/// `ln -s` the real login file into the agent home, for the unsandboxed mode
/// (there is no bind to do it).
fn link_login(spec: &SessionSpec) -> Result<(), String> {
    let Some(login) = &spec.login_file else { return Ok(()) };
    let inside = login_inside(spec.kind, &spec.agent_home.to_string_lossy());
    if let Some(parent) = inside.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    match std::fs::read_link(&inside) {
        Ok(target) if target == *login => return Ok(()),
        Ok(_) => std::fs::remove_file(&inside).map_err(|e| format!("{}: {e}", inside.display()))?,
        Err(_) => {}
    }
    std::os::unix::fs::symlink(login, &inside).map_err(|e| format!("{}: {e}", inside.display()))
}

/// The environment of the agent. `home` is the home as the agent sees it.
fn agent_env(
    kind: AgentKind,
    home: &Path,
    socket: &Path,
    agent_binary: &Path,
    cerbo_bin: &Path,
    host_path: Option<&OsString>,
    extra_path_dirs: &[PathBuf],
) -> Vec<(String, String)> {
    let h = |rel: &str| home.join(rel).to_string_lossy().to_string();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for d in [agent_binary.parent(), cerbo_bin.parent()].into_iter().flatten() {
        dirs.push(d.to_path_buf());
    }
    dirs.extend(extra_path_dirs.iter().cloned());
    if let Some(host) = host_path {
        dirs.extend(std::env::split_paths(host));
    } else {
        for d in ["/run/current-system/sw/bin", "/usr/bin", "/bin"] {
            dirs.push(d.into());
        }
    }
    let mut seen = Vec::new();
    dirs.retain(|d| d.is_absolute() && !seen.contains(d) && {
        seen.push(d.clone());
        true
    });
    let path = std::env::join_paths(&dirs).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

    let mut vars: Vec<(String, String)> = vec![
        ("PATH".into(), path),
        ("HOME".into(), home.to_string_lossy().to_string()),
        ("XDG_CONFIG_HOME".into(), h(".config")),
        ("XDG_DATA_HOME".into(), h(".local/share")),
        ("XDG_STATE_HOME".into(), h(".local/state")),
        ("XDG_CACHE_HOME".into(), h(".cache")),
        ("LANG".into(), "C.UTF-8".into()),
        ("CERBO_SOCKET".into(), socket.to_string_lossy().to_string()),
        ("CERBO_BIN".into(), cerbo_bin.to_string_lossy().to_string()),
    ];
    match kind {
        AgentKind::Opencode => {
            vars.push(("OPENCODE_CONFIG_CONTENT".into(), opencode_config().to_string()));
            vars.push(("OPENCODE_DISABLE_PROJECT_CONFIG".into(), "1".into()));
            vars.push(("OPENCODE_DISABLE_AUTOUPDATE".into(), "1".into()));
            vars.push(("OPENCODE_DISABLE_CLAUDE_CODE".into(), "1".into()));
        }
        AgentKind::Claude => {
            vars.push(("CLAUDE_CONFIG_DIR".into(), h(".claude")));
            vars.push(("DISABLE_AUTOUPDATER".into(), "1".into()));
        }
    }
    vars
}

// ── Binding the agent's own files ────────────────────────────────────────────

fn under_system(path: &Path) -> bool {
    SYSTEM_PATHS.iter().any(|s| path.starts_with(s))
}

/// For a program, the directory to bind read-only (when it is outside the
/// system paths) and the directory to put on `PATH`; plus the same for its
/// `#!` interpreter. `/usr/bin/env node` is resolved to the real `node`.
fn runtime_roots(program: &Path, host_path: Option<&OsString>, fs: &dyn HostFs) -> Vec<(Option<PathBuf>, Option<PathBuf>)> {
    let mut out = Vec::new();
    let real = std::fs::canonicalize(program).unwrap_or_else(|_| program.to_path_buf());
    if !under_system(&real) {
        out.push((Some(install_root(&real)), real.parent().map(Path::to_path_buf)));
    }
    if let Some(interpreter) = shebang_interpreter(&real, host_path) {
        let interpreter = std::fs::canonicalize(&interpreter).unwrap_or(interpreter);
        if !under_system(&interpreter) && fs.exists(&interpreter) {
            out.push((Some(install_root(&interpreter)), interpreter.parent().map(Path::to_path_buf)));
        }
    }
    out
}

/// The directory that holds a program with its libraries: the outermost
/// `node_modules` above it, else its package directory (the parent of `bin`).
fn install_root(real: &Path) -> PathBuf {
    let outer_node_modules = real.ancestors().filter(|a| a.file_name().is_some_and(|n| n == "node_modules")).last();
    if let Some(dir) = outer_node_modules {
        return dir.to_path_buf();
    }
    let parent = real.parent().unwrap_or(real);
    if parent.file_name().is_some_and(|n| n == "bin") {
        return parent.parent().unwrap_or(parent).to_path_buf();
    }
    parent.to_path_buf()
}

fn shebang_interpreter(file: &Path, host_path: Option<&OsString>) -> Option<PathBuf> {
    use std::io::Read;
    let mut head = [0u8; 256];
    let n = std::fs::File::open(file).ok()?.read(&mut head).ok()?;
    let text = std::str::from_utf8(&head[..n]).ok()?;
    let line = text.strip_prefix("#!")?.lines().next()?;
    let mut words = line.split_whitespace();
    let first = words.next()?;
    if Path::new(first).file_name().is_some_and(|n| n == "env") {
        let name = words.find(|w| !w.starts_with('-'))?;
        return agents::which(name, host_path);
    }
    Some(PathBuf::from(first))
}

/// Refuse a root that would hand the agent far more than its own program.
fn check_root(root: &Path, spec: &SessionSpec) -> Result<(), String> {
    let too_wide = root.components().count() < 3
        || spec.real_home.as_deref().is_some_and(|h| h.starts_with(root))
        || spec.vault.starts_with(root)
        || spec.agent_home.starts_with(root)
        || spec.socket_dir.starts_with(root);
    if too_wide {
        return Err(format!(
            "cannot sandbox a program installed in {}: that directory also holds your home, the vault or the agent's state. \
             Install the agent in a directory of its own (or with Nix), or set a path in agents.toml",
            root.display()
        ));
    }
    Ok(())
}

// ── Probe ────────────────────────────────────────────────────────────────────

type ProbeCache = Mutex<HashMap<PathBuf, (Instant, Result<(), String>)>>;

fn cache() -> &'static ProbeCache {
    static CACHE: std::sync::OnceLock<ProbeCache> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Check that `bwrap` can start a sandbox like the real ones, by running a
/// shell in one. A success is remembered for the life of the daemon, a failure
/// for a few seconds, so a fix needs no restart.
pub fn probe(bwrap: &Path) -> Result<(), String> {
    if let Some((at, result)) = cache().lock().unwrap_or_else(|e| e.into_inner()).get(bwrap) {
        if result.is_ok() || at.elapsed() < FAILED_PROBE_TTL {
            return result.clone();
        }
    }
    let result = run_probe(bwrap);
    cache().lock().unwrap_or_else(|e| e.into_inner()).insert(bwrap.to_path_buf(), (Instant::now(), result.clone()));
    result
}

/// The probe's argument list: the base of every sandbox and a shell.
pub fn probe_args(fs: &dyn HostFs) -> Result<Vec<String>, String> {
    let shell = ["/bin/sh", "/usr/bin/sh"]
        .into_iter()
        .find(|s| fs.exists(Path::new(s)))
        .ok_or("agent sandbox unavailable: the host has no /bin/sh to probe with")?;
    let mut a: Vec<String> = ["--unshare-all", "--share-net", "--die-with-parent", "--new-session", "--clearenv", "--dev", "/dev", "--proc", "/proc", "--tmpfs", "/tmp", "--tmpfs", "/home", "--tmpfs", "/run"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    for path in SYSTEM_PATHS {
        let path = Path::new(path);
        match fs.symlink_target(path) {
            Some(target) if !target.is_absolute() && path != Path::new("/run/current-system") => {
                a.extend(["--symlink".to_string(), target.to_string_lossy().to_string(), path.to_string_lossy().to_string()]);
            }
            _ if fs.exists(path) => {
                a.extend(["--ro-bind".to_string(), path.to_string_lossy().to_string(), path.to_string_lossy().to_string()]);
            }
            _ => {}
        }
    }
    a.extend(["--".to_string(), shell.to_string(), "-c".to_string(), "true".to_string()]);
    Ok(a)
}

fn run_probe(bwrap: &Path) -> Result<(), String> {
    if !bwrap.is_file() {
        return Err(format!("agent sandbox unavailable: {} (CERBO_BWRAP) does not exist", bwrap.display()));
    }
    let args = probe_args(&RealFs)?;
    let out = std::process::Command::new(bwrap)
        .args(&args)
        .env_clear()
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("agent sandbox unavailable: cannot run {}: {e}", bwrap.display()))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stderr = stderr.trim();
    let userns = ["Operation not permitted", "No permissions to create new namespace", "setting up uid map", "Permission denied", "uid map"]
        .iter()
        .any(|needle| stderr.contains(needle));
    let hint = if userns {
        " Unprivileged user namespaces are probably disabled or restricted (sysctl kernel.unprivileged_userns_clone, \
         user.max_user_namespaces, or AppArmor's restrict_unprivileged_userns); enable them, or set \
         `[agent] sandbox = \"none\"` in agents.toml to run agents without a sandbox."
    } else {
        ""
    };
    Err(format!("agent sandbox unavailable: a test sandbox with {} failed: {stderr}.{hint}", bwrap.display()))
}

// ── Agent homes and generated config (D10) ───────────────────────────────────

/// The directory of all agent homes: `$XDG_DATA_HOME/cerbo/agents`.
pub fn agents_root(env: &HostEnv) -> Result<PathBuf, String> {
    env.data_home().map(|d| d.join("cerbo/agents")).ok_or_else(|| "neither XDG_DATA_HOME nor HOME is set: no place for the agent homes".to_string())
}

/// The home of one agent kind, `<root>/<agent>/home`.
pub fn home_path(root: &Path, kind: AgentKind) -> PathBuf {
    root.join(agents::name_of(kind)).join("home")
}

/// The tools of Claude Code that change files, run commands or leave the
/// vault. The session is also started with `tools: []` (see
/// [`claude_session_meta`]); this list is the second lock, in the settings
/// file, which is also what the CLI reads if the first is ever dropped.
pub const CLAUDE_DENIED_TOOLS: [&str; 15] = [
    "Bash",
    "BashOutput",
    "KillShell",
    "KillBash",
    "Edit",
    "MultiEdit",
    "Write",
    "NotebookEdit",
    "Read",
    "Glob",
    "Grep",
    "WebFetch",
    "WebSearch",
    "Task",
    "Agent",
];

/// OpenCode's configuration: everything denied, then the `cerbo_*` MCP tools
/// allowed, for the default rules and for the `build` and `plan` agents (the
/// last matching rule wins).
pub fn opencode_config() -> Value {
    let rules = json!([
        { "action": "*", "resource": "*", "effect": "deny" },
        { "action": "cerbo_*", "resource": "*", "effect": "allow" }
    ]);
    json!({
        "permissions": rules,
        "agents": {
            "build": { "permissions": rules },
            "plan": { "permissions": rules }
        }
    })
}

/// The contents of `.claude/settings.json` in the Claude agent home.
pub fn claude_settings() -> Value {
    json!({
        "permissions": {
            "deny": CLAUDE_DENIED_TOOLS,
            "allow": ["mcp__cerbo"],
            "defaultMode": "default"
        }
    })
}

/// The `_meta` for `session/new` and `session/load` with Claude: no built-in
/// tools at all, and only the user settings (the agent home's), so a
/// `.claude/settings.json` inside the vault is ignored. Read from the adapter
/// source: `_meta.claudeCode.options` is spread over the SDK options.
pub fn claude_session_meta() -> Value {
    json!({
        "claudeCode": {
            "options": {
                "tools": [],
                "disallowedTools": CLAUDE_DENIED_TOOLS,
                "settingSources": ["user"]
            }
        }
    })
}

/// The `mcpServers` entry for `session/new`: the `cerbo` binary inside the
/// sandbox, bound to the session socket and the vault.
pub fn mcp_server_entry(cerbo_bin: &Path, vault: &Path) -> Value {
    let socket = Path::new(SANDBOX_SOCKET_DIR).join(SESSION_SOCKET_NAME);
    json!({
        "name": "cerbo",
        "command": cerbo_bin.to_string_lossy(),
        "args": ["mcp", "--vault", vault.to_string_lossy(), "--socket", socket.to_string_lossy()],
        "env": []
    })
}

/// Create the agent's home (mode 0700) and write its generated config. Done
/// at each session start, so a change here takes effect on the next session.
pub fn prepare_home(root: &Path, kind: AgentKind) -> Result<PathBuf, String> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let home = home_path(root, kind);
    let err = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    for dir in [root.to_path_buf(), root.join(agents::name_of(kind)), home.clone()] {
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&dir).map_err(|e| err(&dir, e))?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).map_err(|e| err(&dir, e))?;
    }
    for rel in [".config", ".local/share", ".local/state", ".cache"] {
        let dir = home.join(rel);
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&dir).map_err(|e| err(&dir, e))?;
    }
    match kind {
        AgentKind::Opencode => {
            std::fs::DirBuilder::new().recursive(true).mode(0o700).create(home.join(".local/share/opencode")).map_err(|e| err(&home, e))?;
        }
        AgentKind::Claude => {
            let dir = home.join(".claude");
            std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&dir).map_err(|e| err(&dir, e))?;
            let file = dir.join("settings.json");
            let text = serde_json::to_string_pretty(&claude_settings()).map_err(|e| e.to_string())?;
            cerbo_core::fsio::write_atomic_str(&file, &text).map_err(|e| err(&file, e))?;
        }
    }
    Ok(home)
}

/// The `cerbo` binary the agent runs: `CERBO_BIN` (absolute), else `cerbo`
/// next to the daemon's own executable.
pub fn resolve_cerbo_bin(cerbo_bin_env: Option<OsString>, current_exe: Option<PathBuf>) -> Result<PathBuf, String> {
    let candidate = match cerbo_bin_env.filter(|v| !v.is_empty()) {
        Some(value) => {
            let path = PathBuf::from(value);
            if !path.is_absolute() {
                return Err(format!("CERBO_BIN must be an absolute path, not {}", path.display()));
            }
            path
        }
        None => current_exe
            .and_then(|exe| std::fs::canonicalize(exe).ok())
            .and_then(|exe| exe.parent().map(|d| d.join("cerbo")))
            .ok_or("cannot find the cerbo binary: set CERBO_BIN to its absolute path")?,
    };
    if !candidate.is_file() {
        return Err(format!("cannot find the cerbo binary at {}: set CERBO_BIN to its absolute path", candidate.display()));
    }
    std::fs::canonicalize(&candidate).map_err(|e| format!("{}: {e}", candidate.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    struct FakeFs {
        present: BTreeSet<&'static str>,
        links: Vec<(&'static str, &'static str)>,
    }

    impl HostFs for FakeFs {
        fn exists(&self, path: &Path) -> bool {
            self.present.contains(path.to_str().unwrap())
        }
        fn symlink_target(&self, path: &Path) -> Option<PathBuf> {
            self.links.iter().find(|(p, _)| Path::new(p) == path).map(|(_, t)| PathBuf::from(t))
        }
    }

    fn nixos() -> FakeFs {
        FakeFs { present: ["/nix", "/usr", "/bin", "/etc", "/.host-etc", "/run/current-system", "/bin/sh"].into(), links: vec![] }
    }

    fn fedora() -> FakeFs {
        FakeFs {
            present: ["/usr", "/bin", "/sbin", "/lib", "/lib64", "/etc", "/bin/sh"].into(),
            links: vec![("/bin", "usr/bin"), ("/sbin", "usr/sbin"), ("/lib", "usr/lib"), ("/lib64", "usr/lib64")],
        }
    }

    fn spec() -> SessionSpec {
        SessionSpec {
            kind: AgentKind::Opencode,
            agent_binary: "/nix/store/aaa-opencode/bin/opencode".into(),
            vault: "/srv/notes137".into(),
            socket_dir: "/run/user/1000/cerbo/sessions/s137".into(),
            agent_home: "/data/cerbo/agents/opencode/home".into(),
            cerbo_bin: "/nix/store/bbb-cerbo/bin/cerbo".into(),
            login_file: Some("/real/.local/share/opencode/auth.json".into()),
            real_home: Some("/real".into()),
            host_path: None,
        }
    }

    fn pair_present(args: &[String], flag: &str, a: &str, b: &str) -> bool {
        args.windows(3).any(|w| w[0] == flag && w[1] == a && w[2] == b)
    }

    #[test]
    fn nixos_argv_matches_the_design() {
        let args = bwrap_args(&spec(), &nixos()).unwrap();
        let head: Vec<&str> = args.iter().take(15).map(String::as_str).collect();
        assert_eq!(
            head,
            ["--unshare-all", "--share-net", "--die-with-parent", "--new-session", "--clearenv", "--dev", "/dev", "--proc", "/proc", "--tmpfs", "/tmp", "--tmpfs", "/home", "--tmpfs", "/run"]
        );
        for p in ["/nix", "/usr", "/bin", "/etc", "/.host-etc", "/run/current-system"] {
            assert!(pair_present(&args, "--ro-bind", p, p), "{p} missing in {args:?}");
        }
        assert!(!args.contains(&"/lib64".to_string()), "absent paths are not bound");
        assert!(pair_present(&args, "--ro-bind", "/srv/notes137", "/srv/notes137"));
        assert!(pair_present(&args, "--bind", "/data/cerbo/agents/opencode/home", SANDBOX_HOME));
        assert!(pair_present(&args, "--bind", "/real/.local/share/opencode/auth.json", "/home/agent/.local/share/opencode/auth.json"));
        assert!(pair_present(&args, "--bind", "/run/user/1000/cerbo/sessions/s137", SANDBOX_SOCKET_DIR));
        assert!(args.windows(2).any(|w| w[0] == "--chdir" && w[1] == "/srv/notes137"));
        assert!(pair_present(&args, "--setenv", "CERBO_SOCKET", "/run/cerbo/session/agent.sock"));
        assert!(pair_present(&args, "--setenv", "OPENCODE_DISABLE_PROJECT_CONFIG", "1"));
        assert_eq!(&args[args.len() - 3..], ["--", "/nix/store/aaa-opencode/bin/opencode", "acp"]);
        // The daemon's main socket and the real home are never named.
        assert!(!args.iter().any(|a| a.contains("cerboid.sock")));
        assert!(!args.iter().any(|a| a == "/real"));
        assert_eq!(args.iter().filter(|a| a.starts_with("/real/")).count(), 1, "only the login file of the real home");
    }

    #[test]
    fn plain_hosts_bind_usr_etc_and_keep_usrmerge_links() {
        let args = bwrap_args(&spec(), &fedora()).unwrap();
        assert!(pair_present(&args, "--ro-bind", "/usr", "/usr"));
        assert!(pair_present(&args, "--ro-bind", "/etc", "/etc"));
        assert!(pair_present(&args, "--symlink", "usr/bin", "/bin"));
        assert!(pair_present(&args, "--symlink", "usr/lib64", "/lib64"));
        assert!(!args.contains(&"/nix".to_string()) || args.iter().any(|a| a.starts_with("/nix/store")));
        assert!(!pair_present(&args, "--ro-bind", "/.host-etc", "/.host-etc"));
        assert!(!pair_present(&args, "--ro-bind", "/run/current-system", "/run/current-system"));
    }

    #[test]
    fn claude_gets_its_config_dir_and_credentials_path() {
        let mut s = spec();
        s.kind = AgentKind::Claude;
        s.agent_binary = "/nix/store/ccc-claude-agent-acp/bin/claude-agent-acp".into();
        s.login_file = Some("/real/.claude/.credentials.json".into());
        let args = bwrap_args(&s, &nixos()).unwrap();
        assert!(pair_present(&args, "--bind", "/real/.claude/.credentials.json", "/home/agent/.claude/.credentials.json"));
        assert!(pair_present(&args, "--setenv", "CLAUDE_CONFIG_DIR", "/home/agent/.claude"));
        assert!(!args.iter().any(|a| a == "OPENCODE_CONFIG_CONTENT"));
        assert_eq!(args.last().unwrap(), "/nix/store/ccc-claude-agent-acp/bin/claude-agent-acp");
    }

    #[test]
    fn a_missing_login_file_is_not_bound() {
        let mut s = spec();
        s.login_file = None;
        let args = bwrap_args(&s, &nixos()).unwrap();
        assert!(!args.iter().any(|a| a.ends_with("auth.json")));
    }

    #[test]
    fn a_program_outside_nix_gets_its_own_root_bound_read_only() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg = tmp.path().join("lib/node_modules/@x/adapter/dist");
        std::fs::create_dir_all(&pkg).unwrap();
        let script = pkg.join("index.js");
        std::fs::write(&script, "#!/nonexistent/node\n").unwrap();
        let mut s = spec();
        s.agent_binary = script.clone();
        let args = bwrap_args(&s, &nixos()).unwrap();
        let root = std::fs::canonicalize(tmp.path().join("lib/node_modules")).unwrap();
        let root = root.to_string_lossy().to_string();
        assert!(pair_present(&args, "--ro-bind", &root, &root), "{args:?}");
        assert!(!args.iter().any(|a| a == &tmp.path().to_string_lossy().to_string()));
    }

    #[test]
    fn a_root_that_would_expose_the_home_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = std::fs::canonicalize(tmp.path()).unwrap();
        let bin = dir.join("agent");
        std::fs::write(&bin, "x").unwrap();
        let mut s = spec();
        s.agent_binary = bin;
        s.real_home = Some(dir.clone());
        let err = bwrap_args(&s, &nixos()).unwrap_err();
        assert!(err.contains("cannot sandbox a program installed in"), "{err}");
    }

    #[test]
    fn the_probe_uses_the_same_base() {
        let args = probe_args(&nixos()).unwrap();
        assert!(pair_present(&args, "--ro-bind", "/.host-etc", "/.host-etc"));
        assert_eq!(&args[args.len() - 4..], ["--", "/bin/sh", "-c", "true"]);
        let empty = FakeFs { present: BTreeSet::new(), links: vec![] };
        assert!(probe_args(&empty).unwrap_err().contains("/bin/sh"));
    }

    #[test]
    fn the_sandbox_needs_an_absolute_bwrap_unless_turned_off() {
        use cerbo_core::agent::SandboxMode;
        assert_eq!(confinement(SandboxMode::None, None), Ok(Confinement::None));
        assert!(confinement(SandboxMode::Bwrap, None).unwrap_err().contains("CERBO_BWRAP is not set"));
        assert!(confinement(SandboxMode::Bwrap, Some("bwrap".into())).unwrap_err().contains("absolute"));
        assert!(confinement(SandboxMode::Bwrap, Some("/nonexistent/bwrap".into())).unwrap_err().contains("does not exist"));
    }

    #[test]
    fn a_failing_bwrap_is_explained() {
        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("bwrap");
        std::fs::write(&fake, "#!/bin/sh\necho 'bwrap: No permissions to create new namespace' >&2\nexit 1\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let err = run_probe(&fake).unwrap_err();
        assert!(err.contains("user namespaces"), "{err}");
        assert!(err.contains("sandbox = \"none\""), "{err}");
    }

    #[test]
    fn opencode_denies_everything_but_cerbo_tools() {
        let config = opencode_config();
        let last = |rules: &Value| rules.as_array().unwrap().last().unwrap().clone();
        assert_eq!(config["permissions"][0], json!({ "action": "*", "resource": "*", "effect": "deny" }));
        assert_eq!(last(&config["permissions"]), json!({ "action": "cerbo_*", "resource": "*", "effect": "allow" }));
        assert_eq!(config["agents"]["build"]["permissions"], config["permissions"]);
        assert_eq!(config["agents"]["plan"]["permissions"], config["permissions"]);
    }

    #[test]
    fn claude_is_locked_down_twice() {
        let settings = claude_settings();
        let deny = settings["permissions"]["deny"].as_array().unwrap();
        for tool in ["Bash", "Edit", "Write", "NotebookEdit"] {
            assert!(deny.iter().any(|d| d == tool), "{tool}");
        }
        let meta = claude_session_meta();
        assert_eq!(meta["claudeCode"]["options"]["tools"], json!([]));
        assert_eq!(meta["claudeCode"]["options"]["settingSources"], json!(["user"]));
    }

    #[test]
    fn homes_are_private_and_the_claude_settings_are_rewritten_each_time() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("cerbo/agents");
        let home = prepare_home(&root, AgentKind::Claude).unwrap();
        assert_eq!(home, root.join("claude/home"));
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&home), 0o700);
        assert_eq!(mode(&root), 0o700);
        let file = home.join(".claude/settings.json");
        std::fs::write(&file, "{\"permissions\":{\"allow\":[\"Bash\"]}}").unwrap();
        prepare_home(&root, AgentKind::Claude).unwrap();
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(written, claude_settings());
        let oc = prepare_home(&root, AgentKind::Opencode).unwrap();
        assert!(oc.join(".local/share/opencode").is_dir());
    }

    #[test]
    fn none_mode_runs_the_agent_with_a_warning_and_the_same_variables() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = spec();
        s.agent_home = tmp.path().join("home");
        std::fs::create_dir_all(&s.agent_home).unwrap();
        s.socket_dir = tmp.path().join("sock");
        let login = tmp.path().join("auth.json");
        std::fs::write(&login, "{}").unwrap();
        s.login_file = Some(login.clone());
        let launch = plan(&Confinement::None, &s, &RealFs).unwrap();
        assert!(!launch.sandboxed);
        assert!(launch.warning.unwrap().contains("without a sandbox"));
        assert_eq!(launch.program, s.agent_binary);
        assert_eq!(launch.args, vec![OsString::from("acp")]);
        assert_eq!(launch.cwd.as_deref(), Some(s.vault.as_path()));
        let get = |k: &str| launch.env.iter().find(|(key, _)| key == k).map(|(_, v)| v.to_string_lossy().to_string());
        assert_eq!(get("HOME"), Some(s.agent_home.to_string_lossy().to_string()));
        assert_eq!(get("CERBO_SOCKET"), Some(s.socket_dir.join("agent.sock").to_string_lossy().to_string()));
        assert_eq!(std::fs::read_link(s.agent_home.join(".local/share/opencode/auth.json")).unwrap(), login);
    }

    #[test]
    fn cerbo_bin_comes_from_the_variable_or_the_neighbour() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("cerbo");
        std::fs::write(&bin, "x").unwrap();
        std::fs::write(tmp.path().join("cerboid"), "x").unwrap();
        let real = std::fs::canonicalize(&bin).unwrap();
        assert_eq!(resolve_cerbo_bin(Some(bin.clone().into()), None).unwrap(), real);
        assert_eq!(resolve_cerbo_bin(None, Some(tmp.path().join("cerboid"))).unwrap(), real);
        assert!(resolve_cerbo_bin(Some("cerbo".into()), None).unwrap_err().contains("absolute"));
        assert!(resolve_cerbo_bin(Some("/nonexistent/cerbo".into()), None).is_err());
    }

    #[test]
    fn the_mcp_entry_names_the_session_socket() {
        let entry = mcp_server_entry(Path::new("/nix/store/b/bin/cerbo"), Path::new("/srv/notes137"));
        assert_eq!(entry["command"], "/nix/store/b/bin/cerbo");
        assert_eq!(entry["args"], json!(["mcp", "--vault", "/srv/notes137", "--socket", "/run/cerbo/session/agent.sock"]));
    }
}
