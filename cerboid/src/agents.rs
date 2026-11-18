//! The agent registry (design D11): which agent CLIs this machine has, where,
//! at which version, and whether each is signed in.
//!
//! OpenCode runs as `opencode acp`; Claude Code through the ACP adapter
//! `claude-agent-acp`. Each is looked up once when the daemon starts, on
//! `PATH` or at the absolute path of `agents.toml` (`[agent] opencode = ...`),
//! and resolved to its real path, which is what the sandbox binds.

use cerbo_core::agent::AgentSection;
use cerbo_core::rpc::{AgentInfo, AgentKind};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long `--version` may take before the agent counts as broken.
pub const VERSION_TIMEOUT: Duration = Duration::from_secs(5);

/// The parts of the process environment the registry looks at, so tests can
/// give their own.
#[derive(Debug, Clone, Default)]
pub struct HostEnv {
    pub path: Option<OsString>,
    pub home: Option<PathBuf>,
    pub xdg_data_home: Option<PathBuf>,
    pub claude_config_dir: Option<PathBuf>,
}

impl HostEnv {
    pub fn from_process() -> Self {
        let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
        Self {
            path: var("PATH"),
            home: var("HOME").map(PathBuf::from),
            xdg_data_home: var("XDG_DATA_HOME").map(PathBuf::from),
            claude_config_dir: var("CLAUDE_CONFIG_DIR").map(PathBuf::from),
        }
    }

    /// The user's data directory (`$XDG_DATA_HOME`, else `~/.local/share`).
    pub fn data_home(&self) -> Option<PathBuf> {
        self.xdg_data_home.clone().or_else(|| self.home.as_ref().map(|h| h.join(".local/share")))
    }
}

/// What the registry knows about one agent.
#[derive(Debug, Clone)]
pub struct AgentEntry {
    pub kind: AgentKind,
    /// The real path of the binary, when found.
    pub binary: Option<PathBuf>,
    pub version: Option<String>,
    /// Why it cannot run, when it cannot.
    pub problem: Option<String>,
    /// The file that holds the agent's login on the real home, whether or not
    /// it exists.
    pub login_file: Option<PathBuf>,
}

impl AgentEntry {
    pub fn info(&self) -> AgentInfo {
        AgentInfo {
            agent: self.kind,
            available: self.binary.is_some() && self.problem.is_none(),
            reason: self.problem.clone(),
            path: self.binary.as_ref().map(|p| p.display().to_string()),
            version: self.version.clone(),
            logged_in: self.login_file.as_ref().map(|f| f.is_file()),
        }
    }
}

/// The binary name looked up on `PATH`, and the arguments that start ACP.
pub fn command_of(kind: AgentKind) -> (&'static str, &'static [&'static str]) {
    match kind {
        AgentKind::Opencode => ("opencode", &["acp"]),
        AgentKind::Claude => ("claude-agent-acp", &[]),
    }
}

/// The stable name used in directory names and messages.
pub fn name_of(kind: AgentKind) -> &'static str {
    match kind {
        AgentKind::Opencode => "opencode",
        AgentKind::Claude => "claude",
    }
}

/// Where the agent keeps its login on the real home.
pub fn login_file(kind: AgentKind, env: &HostEnv) -> Option<PathBuf> {
    match kind {
        AgentKind::Opencode => env.data_home().map(|d| d.join("opencode/auth.json")),
        AgentKind::Claude => env
            .claude_config_dir
            .clone()
            .or_else(|| env.home.as_ref().map(|h| h.join(".claude")))
            .map(|d| d.join(".credentials.json")),
    }
}

/// Find `name` in the directories of `path`: the first regular executable.
pub fn which(name: &str, path: Option<&OsString>) -> Option<PathBuf> {
    let path = path?;
    std::env::split_paths(path).filter(|d| d.is_absolute()).map(|d| d.join(name)).find(|c| is_executable(c))
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

/// Run `binary --version` with a scratch `home`, a minimal environment and a
/// time limit; the first line of the output.
pub fn probe_version(binary: &Path, home: &Path, path_env: Option<&OsString>, timeout: Duration) -> Result<String, String> {
    let _ = std::fs::create_dir_all(home);
    let mut command = Command::new(binary);
    command
        .arg("--version")
        .env_clear()
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("XDG_STATE_HOME", home.join(".local/state"))
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .env("CLAUDE_CONFIG_DIR", home.join(".claude"))
        .env("OPENCODE_DISABLE_AUTOUPDATE", "1")
        .env("DISABLE_AUTOUPDATER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(path) = path_env {
        command.env("PATH", path);
    }
    let mut child = command.spawn().map_err(|e| format!("cannot run {}: {e}", binary.display()))?;
    let mut stdout = child.stdout.take().ok_or("no output pipe")?;
    // Read on a thread so a program that never exits cannot block the wait.
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = (&mut stdout).take(4096).read_to_string(&mut text);
        text
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{} --version did not answer within {} s", binary.display(), timeout.as_secs()));
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let text = reader.join().unwrap_or_default();
    if !status.success() {
        return Err(format!("{} --version failed ({status})", binary.display()));
    }
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{} --version printed nothing", binary.display()))
}

/// Look one agent up. `configured` is the absolute path from `agents.toml`.
pub fn detect(kind: AgentKind, configured: Option<&str>, env: &HostEnv, probe_home: &Path, timeout: Duration) -> AgentEntry {
    let (bin_name, _) = command_of(kind);
    let login_file = login_file(kind, env);
    let mut entry = AgentEntry { kind, binary: None, version: None, problem: None, login_file };

    let found = match configured {
        Some(path) => {
            let path = Path::new(path);
            if !path.is_absolute() {
                entry.problem = Some(format!("agents.toml: [agent] {} must be an absolute path, not {}", name_of(kind), path.display()));
                return entry;
            }
            if !is_executable(path) {
                entry.problem = Some(format!("{} (set in agents.toml) is not an executable file", path.display()));
                return entry;
            }
            path.to_path_buf()
        }
        None => match which(bin_name, env.path.as_ref()) {
            Some(path) => path,
            None => {
                entry.problem = Some(format!("{bin_name} was not found on PATH; install it or set [agent] {} in agents.toml", name_of(kind)));
                return entry;
            }
        },
    };
    let real = match std::fs::canonicalize(&found) {
        Ok(real) => real,
        Err(e) => {
            entry.problem = Some(format!("{}: {e}", found.display()));
            return entry;
        }
    };
    entry.binary = Some(real.clone());
    match probe_version(&real, probe_home, env.path.as_ref(), timeout) {
        Ok(version) => entry.version = Some(version),
        Err(e) => entry.problem = Some(e),
    }
    entry
}

/// The registry: both agents, looked up at daemon start.
#[derive(Debug, Clone)]
pub struct AgentRegistry {
    entries: Vec<AgentEntry>,
}

impl AgentRegistry {
    pub fn detect(section: &AgentSection, env: &HostEnv, probe_home: &Path) -> Self {
        let entries = [(AgentKind::Opencode, section.opencode.as_deref()), (AgentKind::Claude, section.claude.as_deref())]
            .into_iter()
            .map(|(kind, configured)| detect(kind, configured, env, &probe_home.join(name_of(kind)), VERSION_TIMEOUT))
            .collect();
        Self { entries }
    }

    pub fn get(&self, kind: AgentKind) -> Option<&AgentEntry> {
        self.entries.iter().find(|e| e.kind == kind)
    }

    /// The answer of `agent_list`.
    pub fn infos(&self) -> Vec<AgentInfo> {
        self.entries.iter().map(AgentEntry::info).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn env_with(path_dir: &Path, home: &Path) -> HostEnv {
        HostEnv {
            path: Some(path_dir.as_os_str().to_owned()),
            home: Some(home.to_path_buf()),
            xdg_data_home: None,
            claude_config_dir: None,
        }
    }

    #[test]
    fn a_missing_agent_says_what_to_do() {
        let tmp = tempfile::tempdir().unwrap();
        let env = env_with(tmp.path(), tmp.path());
        let entry = detect(AgentKind::Opencode, None, &env, &tmp.path().join("p"), Duration::from_secs(2));
        let info = entry.info();
        assert!(!info.available);
        assert!(info.reason.unwrap().contains("not found on PATH"));
        assert_eq!(info.path, None);
    }

    #[test]
    fn a_found_agent_is_resolved_and_probed() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let real = script(tmp.path(), "real-opencode", "echo 'v1.37.0'");
        std::os::unix::fs::symlink(&real, bin.join("opencode")).unwrap();
        let env = env_with(&bin, tmp.path());
        let info = detect(AgentKind::Opencode, None, &env, &tmp.path().join("p"), Duration::from_secs(5)).info();
        assert!(info.available, "{:?}", info.reason);
        assert_eq!(info.version.as_deref(), Some("v1.37.0"));
        assert_eq!(info.path.as_deref(), Some(std::fs::canonicalize(&real).unwrap().to_str().unwrap()));
        assert_eq!(info.logged_in, Some(false));
    }

    #[test]
    fn a_hanging_version_probe_is_cut_off() {
        let tmp = tempfile::tempdir().unwrap();
        let slow = script(tmp.path(), "slow", "while :; do :; done");
        let started = Instant::now();
        let err = probe_version(&slow, &tmp.path().join("h"), None, Duration::from_millis(200)).unwrap_err();
        assert!(err.contains("did not answer"), "{err}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_configured_path_must_be_absolute_and_executable() {
        let tmp = tempfile::tempdir().unwrap();
        let env = env_with(tmp.path(), tmp.path());
        let rel = detect(AgentKind::Claude, Some("claude-agent-acp"), &env, tmp.path(), Duration::from_secs(1)).info();
        assert!(rel.reason.unwrap().contains("absolute"));
        let missing = detect(AgentKind::Claude, Some("/nonexistent/x"), &env, tmp.path(), Duration::from_secs(1)).info();
        assert!(missing.reason.unwrap().contains("not an executable"));
        let ok = script(tmp.path(), "adapter", "echo 0.108.0");
        let found = detect(AgentKind::Claude, Some(ok.to_str().unwrap()), &env, tmp.path(), Duration::from_secs(5)).info();
        assert!(found.available);
    }

    #[test]
    fn the_login_files_follow_the_xdg_and_claude_variables() {
        let mut env = HostEnv { home: Some("/home/u137".into()), ..Default::default() };
        assert_eq!(login_file(AgentKind::Opencode, &env), Some("/home/u137/.local/share/opencode/auth.json".into()));
        assert_eq!(login_file(AgentKind::Claude, &env), Some("/home/u137/.claude/.credentials.json".into()));
        env.xdg_data_home = Some("/xdg".into());
        env.claude_config_dir = Some("/cc".into());
        assert_eq!(login_file(AgentKind::Opencode, &env), Some("/xdg/opencode/auth.json".into()));
        assert_eq!(login_file(AgentKind::Claude, &env), Some("/cc/.credentials.json".into()));
    }

    #[test]
    fn logged_in_follows_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let mut env = env_with(tmp.path(), tmp.path());
        env.xdg_data_home = Some(tmp.path().join("data"));
        let bin = script(tmp.path(), "oc", "echo 1");
        let probe = |env: &HostEnv| detect(AgentKind::Opencode, Some(bin.to_str().unwrap()), env, tmp.path(), Duration::from_secs(5)).info();
        assert_eq!(probe(&env).logged_in, Some(false));
        std::fs::create_dir_all(tmp.path().join("data/opencode")).unwrap();
        std::fs::write(tmp.path().join("data/opencode/auth.json"), "{}").unwrap();
        assert_eq!(probe(&env).logged_in, Some(true));
    }
}
