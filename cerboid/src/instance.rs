//! Single instance and the owner-only socket (design D2).
//!
//! The instance is whoever holds the exclusive lock on `cerboid.lock`, not
//! whoever created the socket file, so a socket left by a crashed daemon is
//! simply replaced: holding the lock proves nobody is behind it.

use cerbo_core::daemon_paths::DaemonPaths;
use std::fs::{File, OpenOptions, TryLockError};
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;

/// First descriptor systemd passes to a socket-activated service
/// (`SD_LISTEN_FDS_START`).
const LISTEN_FDS_START: RawFd = 3;

/// The right to be the daemon. Dropping it releases the lock and removes the
/// socket file.
#[derive(Debug)]
pub struct Instance {
    paths: DaemonPaths,
    _lock: File,
    keep_socket: bool,
}

/// What [`acquire`] found.
#[derive(Debug)]
pub enum Acquired {
    /// Nobody else runs; the caller is the daemon.
    Instance(Instance),
    /// Another daemon of this user holds the lock.
    AlreadyRunning,
}

/// Take the single-instance lock without waiting.
pub fn acquire(paths: &DaemonPaths) -> Result<Acquired, String> {
    paths.prepare_dir()?;
    // Opening must not truncate: a daemon that runs holds the lock and its pid
    // is in the file; only the holder of the lock rewrites it.
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&paths.lock)
        .map_err(|e| format!("cannot open {}: {e}", paths.lock.display()))?;
    match file.try_lock() {
        Ok(()) => {
            // Only the holder of the lock writes it, so the number is the
            // daemon's: it lets `cerbo daemon stop` end a daemon whose
            // protocol it cannot speak. Best effort; the lock is what counts.
            use std::io::Write;
            let _ = file.set_len(0).and_then(|()| (&file).write_all(std::process::id().to_string().as_bytes()));
            Ok(Acquired::Instance(Instance { paths: paths.clone(), _lock: file, keep_socket: false }))
        }
        Err(TryLockError::WouldBlock) => Ok(Acquired::AlreadyRunning),
        Err(TryLockError::Error(e)) => Err(format!("cannot lock {}: {e}", paths.lock.display())),
    }
}

/// The listening socket systemd handed over (socket activation), if any.
///
/// Follows `sd_listen_fds(3)`: `LISTEN_PID` must be this process and
/// `LISTEN_FDS` at least 1; the first socket is descriptor 3. The variables
/// are removed so child processes do not mistake the descriptor for theirs.
pub fn inherited_listener() -> Result<Option<UnixListener>, String> {
    let pid = std::env::var("LISTEN_PID").ok();
    let fds = std::env::var("LISTEN_FDS").ok();
    // SAFETY: called once during start-up, before other threads read the environment.
    unsafe {
        std::env::remove_var("LISTEN_PID");
        std::env::remove_var("LISTEN_FDS");
        std::env::remove_var("LISTEN_FDNAMES");
    }
    listener_from_env(pid.as_deref(), fds.as_deref(), std::process::id(), LISTEN_FDS_START)
}

fn listener_from_env(
    listen_pid: Option<&str>,
    listen_fds: Option<&str>,
    our_pid: u32,
    first_fd: RawFd,
) -> Result<Option<UnixListener>, String> {
    if listen_pid.and_then(|v| v.trim().parse::<u32>().ok()) != Some(our_pid) {
        return Ok(None);
    }
    let count: u32 = listen_fds.and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    if count == 0 {
        return Ok(None);
    }
    if count > 1 {
        return Err(format!("systemd passed {count} sockets, cerboid takes exactly one"));
    }
    // SAFETY: systemd's contract makes this descriptor ours alone; it is wrapped once.
    let listener = unsafe { UnixListener::from_raw_fd(first_fd) };
    // getsockname fails on anything that is not a socket.
    listener
        .local_addr()
        .map_err(|e| format!("descriptor {first_fd} from systemd is not a Unix socket: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    Ok(Some(listener))
}

impl Instance {
    /// Mark the socket as systemd's: it is not removed when the daemon stops,
    /// so the next client can activate the service again.
    pub fn leave_socket_to_systemd(&mut self) {
        self.keep_socket = true;
    }

    /// Listen on the socket, replacing a stale one. The socket is mode 0600 in
    /// a 0700 directory and never bound to a network address.
    pub fn bind(&self) -> Result<UnixListener, String> {
        match std::fs::remove_file(&self.paths.socket) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("cannot remove stale {}: {e}", self.paths.socket.display())),
        }
        let listener = UnixListener::bind(&self.paths.socket)
            .map_err(|e| format!("cannot listen on {}: {e}", self.paths.socket.display()))?;
        std::fs::set_permissions(&self.paths.socket, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("cannot protect {}: {e}", self.paths.socket.display()))?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        Ok(listener)
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // Still holding the lock here, so the socket is ours to remove,
        // unless systemd owns it.
        if !self.keep_socket {
            let _ = std::fs::remove_file(&self.paths.socket);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> (tempfile::TempDir, DaemonPaths) {
        let tmp = tempfile::tempdir().unwrap();
        let paths = DaemonPaths::in_dir(tmp.path().join("cerbo"));
        (tmp, paths)
    }

    #[test]
    fn socket_activation_is_ignored_when_not_for_this_process() {
        assert!(listener_from_env(None, None, 10, 3).unwrap().is_none());
        assert!(listener_from_env(Some("11"), Some("1"), 10, 3).unwrap().is_none());
        assert!(listener_from_env(Some("10"), Some("0"), 10, 3).unwrap().is_none());
        assert!(listener_from_env(Some("x"), Some("1"), 10, 3).unwrap().is_none());
    }

    #[test]
    fn socket_activation_rejects_several_sockets() {
        assert!(listener_from_env(Some("10"), Some("2"), 10, 3).is_err());
    }

    #[test]
    fn socket_activation_takes_over_the_passed_socket() {
        use std::os::fd::IntoRawFd;
        let (_tmp, paths) = paths();
        paths.prepare_dir().unwrap();
        // systemd binds the socket; the daemon only receives the descriptor.
        let fd = UnixListener::bind(&paths.socket).unwrap().into_raw_fd();

        let listener = listener_from_env(Some("10"), Some("1"), 10, fd).unwrap().expect("inherited");
        let client = std::os::unix::net::UnixStream::connect(&paths.socket).unwrap();
        drop(client);
        loop {
            match listener.accept() {
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5))
                }
                Err(e) => panic!("{e}"),
            }
        }
    }

    #[test]
    fn socket_activation_refuses_a_descriptor_that_is_no_socket() {
        use std::os::fd::IntoRawFd;
        let file = std::fs::File::open("/dev/null").unwrap().into_raw_fd();
        assert!(listener_from_env(Some("10"), Some("1"), 10, file).is_err());
    }

    #[test]
    fn a_socket_left_to_systemd_survives_the_instance() {
        let (_tmp, paths) = paths();
        let Acquired::Instance(mut instance) = acquire(&paths).unwrap() else { panic!("no instance") };
        let _listener = instance.bind().unwrap();
        instance.leave_socket_to_systemd();
        drop(instance);
        assert!(paths.socket.exists());
    }

    fn mode(path: &std::path::Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn a_second_instance_is_told_one_runs() {
        let (_tmp, paths) = paths();
        let first = acquire(&paths).unwrap();
        assert!(matches!(first, Acquired::Instance(_)));
        assert!(matches!(acquire(&paths).unwrap(), Acquired::AlreadyRunning));

        drop(first);
        assert!(matches!(acquire(&paths).unwrap(), Acquired::Instance(_)));
    }

    #[test]
    fn the_socket_is_owner_only_in_an_owner_only_directory() {
        let (_tmp, paths) = paths();
        let Acquired::Instance(instance) = acquire(&paths).unwrap() else { panic!("no instance") };
        let _listener = instance.bind().unwrap();

        assert_eq!(mode(&paths.dir), 0o700);
        assert_eq!(mode(&paths.socket), 0o600);
    }

    #[test]
    fn a_stale_socket_is_replaced() {
        let (_tmp, paths) = paths();
        paths.prepare_dir().unwrap();
        // A crashed daemon leaves its socket file behind.
        drop(UnixListener::bind(&paths.socket).unwrap());
        assert!(paths.socket.exists());

        let Acquired::Instance(instance) = acquire(&paths).unwrap() else { panic!("no instance") };
        let _listener = instance.bind().expect("a stale socket must not block a start");
        assert!(std::os::unix::net::UnixStream::connect(&paths.socket).is_ok());
    }

    #[test]
    fn dropping_the_instance_removes_the_socket() {
        let (_tmp, paths) = paths();
        let Acquired::Instance(instance) = acquire(&paths).unwrap() else { panic!("no instance") };
        let listener = instance.bind().unwrap();
        drop(listener);
        drop(instance);
        assert!(!paths.socket.exists());
    }
}
