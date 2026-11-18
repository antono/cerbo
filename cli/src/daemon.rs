//! The CLI's side of the daemon: `cerbo daemon`, `cerbo approvals`, and the
//! daemon-first path of the existing commands (design D12).
//!
//! A command first tries the daemon and, when none runs, does its work in
//! this process as before. `CERBO_NO_DAEMON=1` skips the daemon altogether.

use cerbo_core::agent::{self, AgentPermission};
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::{Client, ConnectError};
use cerbo_core::rpc::{self, ApprovalInfo, Call, ClientKind, StatusInfo, CODE_UNKNOWN_VAULT};
use cerbo_core::CerboContext;
use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, Instant};

/// Set to anything non-empty to make commands skip the daemon.
pub const NO_DAEMON_ENV: &str = "CERBO_NO_DAEMON";

pub fn paths(ctx: &CerboContext) -> DaemonPaths {
    DaemonPaths::for_user(&ctx.cache_dir)
}

fn skip_daemon() -> bool {
    std::env::var_os(NO_DAEMON_ENV).is_some_and(|v| !v.is_empty())
}

/// A connection to the running daemon; none when no daemon answers. A daemon
/// of another protocol version is reported on stderr and treated as absent, so
/// the command still works on the files.
fn connect(ctx: &CerboContext) -> Option<Client> {
    if skip_daemon() {
        return None;
    }
    match Client::connect(&paths(ctx).socket, ClientKind::Cli, Some("cerbo")) {
        Ok(client) => Some(client),
        Err(ConnectError::Unavailable(_)) => None,
        Err(e) => {
            eprintln!("Warning: not using the daemon: {e}");
            None
        }
    }
}

/// Run the call built for the vault at `vault_root` on the daemon.
///
/// `None` means "do it yourself": no daemon runs, or it does not serve this
/// vault (not registered). `Some(Err)` is the daemon's own answer, a failure of
/// the operation.
pub fn vault_call(
    ctx: &CerboContext,
    vault_root: &Path,
    build: impl FnOnce(String) -> Call,
) -> Option<Result<Value, String>> {
    let mut client = connect(ctx)?;
    let selector = std::fs::canonicalize(vault_root).unwrap_or_else(|_| vault_root.to_path_buf());
    match client.call(build(selector.to_string_lossy().into_owned())) {
        Ok(value) => Some(Ok(value)),
        Err(e) if e.code == CODE_UNKNOWN_VAULT => None,
        Err(e) => Some(Err(e.message)),
    }
}

// ── cerbo daemon ─────────────────────────────────────────────────────────────

fn running(ctx: &CerboContext) -> Result<Option<Client>, String> {
    match Client::connect(&paths(ctx).socket, ClientKind::Cli, Some("cerbo")) {
        Ok(client) => Ok(Some(client)),
        Err(ConnectError::Unavailable(_)) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn start(ctx: &CerboContext, json: bool) -> Result<(), String> {
    let paths = paths(ctx);
    let (client, started) = match running(ctx)? {
        Some(client) => (client, false),
        None => (rpc::launch::start_daemon(&paths, ClientKind::Cli, Some("cerbo"))?, true),
    };
    let pid = client.welcome().pid;
    if json {
        println!("{}", json!({ "running": true, "started": started, "pid": pid, "socket": paths.socket }));
    } else if started {
        println!("Started cerboid (pid {pid})");
    } else {
        println!("cerboid is already running (pid {pid})");
    }
    Ok(())
}

pub fn stop(ctx: &CerboContext, json: bool) -> Result<(), String> {
    let Some(mut client) = running(ctx)? else {
        if json {
            println!("{}", json!({ "running": false, "stopped": false }));
        } else {
            println!("cerboid is not running");
        }
        return Ok(());
    };
    let pid = client.welcome().pid;
    client.call(Call::Stop).map_err(|e| e.message)?;
    drop(client);
    // The daemon finishes queued writes first; wait until it is really gone.
    let socket = paths(ctx).socket;
    let deadline = Instant::now() + Duration::from_secs(40);
    while Client::connect(&socket, ClientKind::Cli, Some("cerbo")).is_ok() {
        if Instant::now() >= deadline {
            return Err("cerboid was told to stop but is still running".to_string());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    // The socket goes before the process does; where /proc tells, wait for the
    // process too, so `stop` then `start` never meets the old daemon's lock.
    let proc = std::path::PathBuf::from(format!("/proc/{pid}"));
    while Path::new("/proc/self").exists() && proc.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    if json {
        println!("{}", json!({ "running": false, "stopped": true, "pid": pid }));
    } else {
        println!("Stopped cerboid (pid {pid})");
    }
    Ok(())
}

fn duration_text(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m {}s", secs / 60, secs % 60),
        _ => format!("{}h {}m", secs / 3600, secs % 3600 / 60),
    }
}

pub fn status(ctx: &CerboContext, json: bool) -> Result<(), String> {
    let paths = paths(ctx);
    let Some(mut client) = running(ctx)? else {
        if json {
            println!("{}", json!({ "running": false, "socket": paths.socket }));
        } else {
            println!("cerboid is not running");
        }
        // Like `systemctl status`: scripts can tell by the exit code.
        std::process::exit(1);
    };
    let info: StatusInfo =
        serde_json::from_value(client.call(Call::Status).map_err(|e| e.message)?).map_err(|e| e.to_string())?;
    let pending = client
        .call(Call::ApprovalList { vault: None })
        .ok()
        .and_then(|v| serde_json::from_value::<Vec<ApprovalInfo>>(v).ok())
        .map_or(0, |list| list.len());
    if json {
        let vaults: Vec<Value> = info
            .vaults
            .iter()
            .map(|v| {
                json!({
                    "id": v.id, "name": v.name, "path": v.path,
                    "agent": agent::get_permission(ctx, &v.id).as_str(),
                })
            })
            .collect();
        println!(
            "{}",
            json!({
                "running": true, "version": info.version, "protocol": info.protocol, "pid": info.pid,
                "uptime_secs": info.uptime_secs, "memory_kb": info.memory_kb, "socket": paths.socket,
                "pending_approvals": pending, "vaults": vaults,
            })
        );
        return Ok(());
    }
    let memory = info.memory_kb.map(|kb| format!(", {} MiB", kb / 1024)).unwrap_or_default();
    println!("cerboid {} running (pid {}, up {}{memory})", info.version, info.pid, duration_text(info.uptime_secs));
    println!("Socket: {}", paths.socket.display());
    if pending > 0 {
        println!("Pending approvals: {pending} (see `cerbo approvals list`)");
    }
    println!("Vaults ({}):", info.vaults.len());
    for v in &info.vaults {
        println!("  {}: {}  [agent: {}]", v.id, v.name, agent::get_permission(ctx, &v.id));
    }
    Ok(())
}

// ── cerbo approvals ──────────────────────────────────────────────────────────

fn need_daemon(ctx: &CerboContext) -> Result<Client, String> {
    running(ctx)?.ok_or_else(|| "cerboid is not running, so there are no pending approvals".to_string())
}

/// `HH:MM:SS` UTC of a Unix time; enough to tell requests apart.
fn time_text(unix: u64) -> String {
    let day = unix % 86_400;
    format!("{:02}:{:02}:{:02}Z", day / 3600, day % 3600 / 60, day % 60)
}

pub fn approvals_list(ctx: &CerboContext, vault: Option<String>, json: bool) -> Result<(), String> {
    let mut client = need_daemon(ctx)?;
    let value = client.call(Call::ApprovalList { vault }).map_err(|e| e.message)?;
    if json {
        println!("{value}");
        return Ok(());
    }
    let list: Vec<ApprovalInfo> = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if list.is_empty() {
        println!("No pending approvals");
    }
    for a in list {
        println!(
            "{}: {} wants to {} in \"{}\" (asked {}, expires {})",
            a.id,
            a.agent,
            a.action,
            a.vault_name,
            time_text(a.requested),
            time_text(a.deadline)
        );
        println!("  {}", a.summary);
        if let Some(diff) = &a.diff {
            for line in diff.lines() {
                println!("    {line}");
            }
        }
    }
    Ok(())
}

pub fn approvals_answer(ctx: &CerboContext, id: u64, approve: bool, json: bool) -> Result<(), String> {
    let mut client = need_daemon(ctx)?;
    client.call(Call::ApprovalAnswer { approval: id, approve }).map_err(|e| e.message)?;
    let word = if approve { "Approved" } else { "Rejected" };
    if json {
        println!("{}", json!({ "success": true, "approval": id, "approved": approve }));
    } else {
        println!("{word} request {id}");
    }
    Ok(())
}

pub fn changes_log(ctx: &CerboContext, vault: &str, limit: usize, json: bool) -> Result<(), String> {
    let vault = find_vault(ctx, vault)?;
    let entries = agent::read_changes(ctx, &vault.id, limit)?;
    if json {
        println!("{}", serde_json::to_string(&entries).map_err(|e| e.to_string())?);
        return Ok(());
    }
    if entries.is_empty() {
        println!("No agent changes recorded for \"{}\"", vault.name);
    }
    for e in entries {
        let how = if e.approved { "approved" } else { "auto" };
        println!("{}  {}  {} {} ({how})", time_text(e.time), e.agent, e.action, e.page);
    }
    Ok(())
}

// ── cerbo vault agent ────────────────────────────────────────────────────────

/// A registered vault by id, name or path.
pub fn find_vault(ctx: &CerboContext, selector: &str) -> Result<cerbo_core::vault::Vault, String> {
    let vaults = cerbo_core::vault::vault_list(ctx)?.vaults;
    let wanted = std::fs::canonicalize(selector).ok();
    vaults
        .iter()
        .find(|v| v.id == selector)
        .or_else(|| vaults.iter().find(|v| v.name == selector))
        .or_else(|| vaults.iter().find(|v| wanted.is_some() && std::fs::canonicalize(&v.path).ok() == wanted))
        .cloned()
        .ok_or_else(|| format!("vault not found: {selector} (see `cerbo vault list`)"))
}

pub fn vault_agent(
    ctx: &CerboContext,
    vault: &str,
    level: Option<AgentPermission>,
    json: bool,
) -> Result<(), String> {
    // Works on agents.toml directly and never contacts the daemon, which
    // picks the level up on its next write; so it is safe in the daemon's own
    // ExecStartPre. A vault that is not registered exits 2, apart from other
    // failures (1), so a caller can tell the two apart.
    let vault = match find_vault(ctx, vault) {
        Ok(vault) => vault,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(2);
        }
    };
    if let Some(level) = level {
        agent::set_permission(ctx, &vault.id, level)?;
    }
    let current = agent::get_permission(ctx, &vault.id);
    if json {
        println!("{}", json!({ "vault": vault.id, "name": vault.name, "agent": current.as_str() }));
    } else if level.is_some() {
        println!("Agent permission of \"{}\" is now {current}", vault.name);
    } else {
        println!("{current}");
    }
    Ok(())
}
