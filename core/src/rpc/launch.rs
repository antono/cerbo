//! Finding and starting the daemon (design D9: connect first, start only if
//! nothing answers, so no path can create two daemons; the daemon's own
//! single-instance lock settles a race).

use super::client::{Client, ConnectError};
use super::{ClientKind, RpcError};
use crate::daemon_paths::DaemonPaths;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Environment variable naming the `cerboid` binary to start.
pub const DAEMON_BIN_ENV: &str = "CERBOID_BIN";

/// How long a started daemon gets to accept connections.
const START_TIMEOUT: Duration = Duration::from_secs(10);

/// The `cerboid` binary: `$CERBOID_BIN`, else the one beside the running
/// program, else the first on `PATH`.
pub fn find_daemon_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(DAEMON_BIN_ENV).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(path));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(sibling) = exe.parent().map(|dir| dir.join("cerboid"))
        && sibling.is_file()
    {
        return Some(sibling);
    }
    std::env::split_paths(&std::env::var_os("PATH")?).map(|dir| dir.join("cerboid")).find(|p| p.is_file())
}

/// Start `cerboid` detached from this process (its own process group, no
/// terminal) and wait until it accepts connections.
pub fn start_daemon(
    paths: &DaemonPaths,
    kind: ClientKind,
    name: Option<&str>,
    vault: Option<&str>,
) -> Result<Client, RpcError> {
    let bin = find_daemon_binary().ok_or_else(|| {
        RpcError::failed(format!(
            "cannot start the daemon: no `cerboid` next to this program or in PATH (set {DAEMON_BIN_ENV})"
        ))
    })?;
    let spawn = || {
        Command::new(&bin)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|e| RpcError::failed(format!("cannot start {}: {e}", bin.display())))
    };
    let mut child = spawn()?;

    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        match Client::connect_full(&paths.socket, kind, name, vault, super::PROTOCOL_VERSION) {
            Ok(client) => return Ok(client),
            Err(ConnectError::Unavailable(_)) => {}
            Err(ConnectError::Rejected(error) | ConnectError::VersionMismatch(error)) => return Err(error),
            Err(other) => return Err(RpcError::failed(other.to_string())),
        }
        // A start that failed ends the process. One that exited cleanly lost
        // the race to a daemon that was still shutting down (it held the
        // instance lock a moment longer than its socket): start again.
        if let Ok(Some(status)) = child.try_wait() {
            if !status.success() {
                return Err(RpcError::failed(format!(
                    "{} stopped at start-up ({status}); run it in a terminal to see why",
                    bin.display()
                )));
            }
            std::thread::sleep(Duration::from_millis(100));
            child = spawn()?;
        }
        if Instant::now() >= deadline {
            return Err(RpcError::failed(format!(
                "the daemon did not accept connections within {}s",
                START_TIMEOUT.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Connect to the running daemon, or start one and connect to that.
pub fn connect_or_start(
    paths: &DaemonPaths,
    kind: ClientKind,
    name: Option<&str>,
    vault: Option<&str>,
) -> Result<Client, RpcError> {
    match Client::connect_full(&paths.socket, kind, name, vault, super::PROTOCOL_VERSION) {
        Ok(client) => Ok(client),
        Err(ConnectError::Unavailable(_)) => start_daemon(paths, kind, name, vault),
        Err(ConnectError::Rejected(error) | ConnectError::VersionMismatch(error)) => Err(error),
        Err(other) => Err(RpcError::failed(other.to_string())),
    }
}
