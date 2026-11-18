//! Where the daemon's socket and single-instance lock live (design D2).
//!
//! `$XDG_RUNTIME_DIR/cerbo/` holds `cerboid.sock` and `cerboid.lock`. The
//! directory is mode 0700 and the socket mode 0600, so only the owner can
//! reach it. Without `XDG_RUNTIME_DIR` (no login session) the directory is
//! `run/` under the cache directory, which is the owner's as well.

use std::path::{Path, PathBuf};

/// The files of one user's daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonPaths {
    /// The 0700 directory holding both files.
    pub dir: PathBuf,
    pub socket: PathBuf,
    /// Held exclusively by the running daemon; its owner is the single instance.
    pub lock: PathBuf,
}

impl DaemonPaths {
    /// The files inside `dir`, wherever it is.
    pub fn in_dir(dir: PathBuf) -> Self {
        Self { socket: dir.join("cerboid.sock"), lock: dir.join("cerboid.lock"), dir }
    }

    /// The user's real location: `$XDG_RUNTIME_DIR/cerbo`, else `<cache_dir>/run`.
    pub fn for_user(cache_dir: &Path) -> Self {
        match std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()) {
            Some(runtime) => Self::in_dir(PathBuf::from(runtime).join("cerbo")),
            None => Self::in_dir(cache_dir.join("run")),
        }
    }

    /// Create the directory with mode 0700, or tighten an existing one to it.
    #[cfg(unix)]
    pub fn prepare_dir(&self) -> Result<(), String> {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let err = |e: std::io::Error| format!("{}: {e}", self.dir.display());
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&self.dir).map_err(err)?;
        // `recursive` leaves an existing directory as it was.
        std::fs::set_permissions(&self.dir, std::fs::Permissions::from_mode(0o700)).map_err(err)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn the_directory_is_owner_only_even_when_it_existed_wider() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = DaemonPaths::in_dir(tmp.path().join("run").join("cerbo"));

        paths.prepare_dir().unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&paths.dir), 0o700);

        std::fs::set_permissions(&paths.dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        paths.prepare_dir().unwrap();
        assert_eq!(mode(&paths.dir), 0o700);
    }

    #[test]
    fn socket_and_lock_sit_beside_each_other() {
        let paths = DaemonPaths::in_dir(PathBuf::from("/run/user/137/cerbo"));
        assert_eq!(paths.socket, PathBuf::from("/run/user/137/cerbo/cerboid.sock"));
        assert_eq!(paths.lock, PathBuf::from("/run/user/137/cerbo/cerboid.lock"));
    }
}
