//! The CLI's side of the daemon: `cerbo daemon`, `cerbo approvals`, and the
//! daemon-first path of the existing commands (design D12).
//!
//! A command first tries the daemon and, when none runs, does its work in
//! this process as before. `CERBO_NO_DAEMON=1` skips the daemon altogether.

use cerbo_core::agent::{self, AgentPermission};
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::{self, AdminError, Client, ConnectError};
use cerbo_core::rpc::{self, AdminAction, ApprovalInfo, Call, ClientKind, StatusInfo, CODE_UNKNOWN_VAULT};
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

/// How long a command waits for a daemon that runs (it holds the lock) but
/// does not take connections yet or any more, such as one that is starting
/// or stopping, before it gives up.
const BUSY_WAIT: Duration = Duration::from_secs(3);

/// A connection to the running daemon; `Ok(None)` only when no daemon runs
/// (nobody holds its lock), so the command may work on the files. A daemon
/// that runs but speaks another protocol version, or does not answer, is an
/// error: writing the files behind its back would break its write queue.
fn connect(ctx: &CerboContext) -> Result<Option<Client>, String> {
    if skip_daemon() {
        return Ok(None);
    }
    let paths = paths(ctx);
    let deadline = Instant::now() + BUSY_WAIT;
    loop {
        match Client::connect(&paths.socket, ClientKind::Cli, Some("cerbo")) {
            Ok(client) => return Ok(Some(client)),
            Err(ConnectError::Unavailable(why)) => {
                if !paths.daemon_alive() {
                    return Ok(None);
                }
                if Instant::now() >= deadline {
                    return Err(format!(
                        "the daemon is running but does not answer ({why}); try again, or restart it with \
                         `cerbo daemon stop` and `cerbo daemon start` (CERBO_NO_DAEMON=1 skips it)"
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("cannot use the running daemon: {e}")),
        }
    }
}

/// Run the call built for the vault at `vault_root` on the daemon.
///
/// `None` means "do it yourself": no daemon runs, or it does not serve this
/// vault (not registered). `Some(Err)` is the daemon's own answer, a failure of
/// the operation, or the reason the daemon that runs cannot be used (another
/// protocol version, not answering): never a fallback to the files.
pub fn vault_call(
    ctx: &CerboContext,
    vault_root: &Path,
    build: impl FnOnce(String) -> Call,
) -> Option<Result<Value, String>> {
    let mut client = match connect(ctx) {
        Ok(client) => client?,
        Err(message) => return Some(Err(message)),
    };
    let selector = std::fs::canonicalize(vault_root).unwrap_or_else(|_| vault_root.to_path_buf());
    match client.call(build(selector.to_string_lossy().into_owned())) {
        Ok(value) => Some(Ok(value)),
        Err(e) if e.code == CODE_UNKNOWN_VAULT => None,
        Err(e) => Some(Err(e.message)),
    }
}

// ── cerbo daemon ─────────────────────────────────────────────────────────────

/// The running daemon, `Ok(None)` when none answers. A daemon of another
/// protocol version is [`ConnectError::VersionMismatch`], which `stop` and
/// `status` handle through the version-independent administration line.
fn try_running(ctx: &CerboContext) -> Result<Option<Client>, ConnectError> {
    match Client::connect(&paths(ctx).socket, ClientKind::Cli, Some("cerbo")) {
        Ok(client) => Ok(Some(client)),
        Err(ConnectError::Unavailable(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

fn running(ctx: &CerboContext) -> Result<Option<Client>, String> {
    try_running(ctx).map_err(|e| e.to_string())
}

/// The process id of a daemon whose protocol this program cannot speak and
/// that does not know administration lines either (it is older than they
/// are): the number the daemon wrote into its lock file, else, on Linux, the
/// process behind the socket when it is a `cerboid` (under systemd socket
/// activation that is systemd's, which is never signalled).
fn foreign_pid(paths: &DaemonPaths) -> Option<u32> {
    paths.lock_pid().or_else(|| socket_peer_pid(&paths.socket))
}

#[cfg(target_os = "linux")]
fn socket_peer_pid(socket: &Path) -> Option<u32> {
    use std::os::fd::AsRawFd;
    let stream = std::os::unix::net::UnixStream::connect(socket).ok()?;
    let mut cred = libc::ucred { pid: 0, uid: 0, gid: 0 };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: `cred` and `len` are valid for the call and `len` is its size.
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast::<libc::c_void>(),
            &mut len,
        )
    };
    if rc != 0 || cred.pid <= 1 {
        return None;
    }
    let pid = cred.pid as u32;
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    (comm.trim() == "cerboid").then_some(pid)
}

#[cfg(not(target_os = "linux"))]
fn socket_peer_pid(_socket: &Path) -> Option<u32> {
    None
}

/// Wait until the daemon's lock is free, then (where `/proc` tells) until its
/// process is gone, so `stop` then `start` never meets the old daemon's lock.
fn wait_gone(paths: &DaemonPaths, pid: u32, deadline: Instant) -> Result<(), String> {
    while paths.daemon_alive() {
        if Instant::now() >= deadline {
            return Err("cerboid was told to stop but is still running".to_string());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let proc = std::path::PathBuf::from(format!("/proc/{pid}"));
    while Path::new("/proc/self").exists() && proc.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

/// Stop a daemon of another protocol version: ask it with the
/// version-independent administration line, and when it is too old to know
/// that, end it with SIGTERM, which it handles like a stop.
fn stop_foreign(ctx: &CerboContext, json: bool) -> Result<(), String> {
    let paths = paths(ctx);
    let (pid, signalled) = match client::admin(&paths.socket, AdminAction::Stop) {
        Ok(info) => (info.pid, false),
        Err(AdminError::Unsupported) => {
            let pid = foreign_pid(&paths).ok_or_else(|| {
                "cerboid runs with another protocol version and is too old to be asked to stop, and its process id \
                 is unknown: end the `cerboid` process yourself (for example `pkill cerboid`)"
                    .to_string()
            })?;
            // SAFETY: plain `kill(2)` on a process id; no memory is involved.
            if unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) } != 0 {
                return Err(format!("cannot stop cerboid (pid {pid}): {}", std::io::Error::last_os_error()));
            }
            (pid, true)
        }
        Err(e) => return Err(e.to_string()),
    };
    wait_gone(&paths, pid, Instant::now() + Duration::from_secs(40))?;
    if json {
        println!("{}", json!({ "running": false, "stopped": true, "pid": pid, "signalled": signalled }));
    } else {
        println!("Stopped cerboid (pid {pid}), which spoke another protocol version");
    }
    Ok(())
}

/// `status` of a daemon of another protocol version: what its administration
/// line says, or only its process id when it is too old for that.
fn status_foreign(ctx: &CerboContext, json: bool) -> Result<(), String> {
    let paths = paths(ctx);
    let (info, pid) = match client::admin(&paths.socket, AdminAction::Status) {
        Ok(info) => {
            let pid = info.pid;
            (Some(info), Some(pid))
        }
        Err(AdminError::Unsupported) => (None, foreign_pid(&paths)),
        Err(e) => return Err(e.to_string()),
    };
    if json {
        println!(
            "{}",
            json!({
                "running": true, "protocol_mismatch": true, "protocol": info.as_ref().map(|i| i.protocol),
                "expected_protocol": rpc::PROTOCOL_VERSION, "version": info.as_ref().map(|i| i.version.clone()),
                "pid": pid, "uptime_secs": info.as_ref().map(|i| i.uptime_secs), "socket": paths.socket,
            })
        );
        return Ok(());
    }
    let pid_text = pid.map_or_else(|| "pid unknown".to_string(), |p| format!("pid {p}"));
    match &info {
        Some(i) => println!(
            "cerboid {} running ({pid_text}, up {}) but speaks protocol {}, this cerbo speaks {}",
            i.version,
            duration_text(i.uptime_secs),
            i.protocol,
            rpc::PROTOCOL_VERSION
        ),
        None => println!(
            "cerboid running ({pid_text}) but speaks another protocol version than this cerbo ({})",
            rpc::PROTOCOL_VERSION
        ),
    }
    println!("Run `cerbo daemon stop`, then `cerbo daemon start`.");
    Ok(())
}

pub fn start(ctx: &CerboContext, json: bool) -> Result<(), String> {
    let paths = paths(ctx);
    let (client, started) = match running(ctx)? {
        Some(client) => (client, false),
        None => (
            rpc::launch::start_daemon(&paths, ClientKind::Cli, Some("cerbo"), None).map_err(|e| e.message)?,
            true,
        ),
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
    let client = match try_running(ctx) {
        Ok(client) => client,
        Err(ConnectError::VersionMismatch(_)) => return stop_foreign(ctx, json),
        Err(e) => return Err(e.to_string()),
    };
    let Some(mut client) = client else {
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
    let client = match try_running(ctx) {
        Ok(client) => client,
        Err(ConnectError::VersionMismatch(_)) => return status_foreign(ctx, json),
        Err(e) => return Err(e.to_string()),
    };
    let Some(mut client) = client else {
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
    let vault = find_vault(ctx, vault)?;
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
