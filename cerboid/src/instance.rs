//! Single instance and the owner-only socket (design D2).
//!
//! The instance is whoever holds the exclusive lock on `cerboid.lock`, not
//! whoever created the socket file, so a socket left by a crashed daemon is
//! simply replaced: holding the lock proves nobody is behind it.

use cerbo_core::daemon_paths::DaemonPaths;
use std::fs::{File, OpenOptions, TryLockError};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;

/// The right to be the daemon. Dropping it releases the lock and removes the
/// socket file.
#[derive(Debug)]
pub struct Instance {
    paths: DaemonPaths,
    _lock: File,
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
    // The lock file is only a handle, never data: opening must not truncate.
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&paths.lock)
        .map_err(|e| format!("cannot open {}: {e}", paths.lock.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(Acquired::Instance(Instance { paths: paths.clone(), _lock: file })),
        Err(TryLockError::WouldBlock) => Ok(Acquired::AlreadyRunning),
        Err(TryLockError::Error(e)) => Err(format!("cannot lock {}: {e}", paths.lock.display())),
    }
}

impl Instance {
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
        // Still holding the lock here, so the socket is ours to remove.
        let _ = std::fs::remove_file(&self.paths.socket);
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
