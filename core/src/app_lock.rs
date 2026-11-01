//! The advisory lock that keeps automatic vault rewrites away from a running
//! desktop app.
//!
//! The desktop app edits `meta.ttl` and `page.md` while it runs, and a vault
//! rewrite that read a file just before the app saved it would put the old
//! content back. So every running app holds this lock *shared* for its whole
//! life, and anything that rewrites vault files on its own initiative — the
//! automatic vocabulary migration of the CLI and of the desktop launcher, and
//! `cerbo migrate vocabulary` — first takes it *exclusively*, without waiting.
//! When it cannot, the rewrite is skipped (or refused) and nothing is written.
//!
//! The lock is one file per user, `$XDG_CACHE_HOME/cerbo/desktop.lock`, since
//! one app can open any of the user's vaults. It is `std::fs::File`'s `flock`,
//! released when the holder drops it or dies.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

/// The lock file under `cache_dir` (`$XDG_CACHE_HOME/cerbo`).
pub fn lock_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("desktop.lock")
}

/// A held app lock. Dropping it releases the lock.
#[derive(Debug)]
pub struct AppLock {
    _file: File,
}

impl AppLock {
    /// Hold the lock shared, as a running desktop app does. Waits while a
    /// rewrite holds it exclusively, which lasts only as long as the rewrite.
    pub fn shared(cache_dir: &Path) -> Result<Self, String> {
        let file = open(cache_dir)?;
        file.lock_shared()
            .map_err(|e| format!("cannot lock {}: {e}", lock_path(cache_dir).display()))?;
        Ok(Self { _file: file })
    }

    /// Take the lock exclusively without waiting. `None` when a desktop app
    /// (or another rewrite) holds it.
    pub fn try_exclusive(cache_dir: &Path) -> Result<Option<Self>, String> {
        let file = open(cache_dir)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Self { _file: file })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(e)) => {
                Err(format!("cannot lock {}: {e}", lock_path(cache_dir).display()))
            }
        }
    }
}

fn open(cache_dir: &Path) -> Result<File, String> {
    std::fs::create_dir_all(cache_dir).map_err(|e| format!("{}: {e}", cache_dir.display()))?;
    let path = lock_path(cache_dir);
    // Opening never truncates: the file is only a lock handle, never data.
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_running_app_keeps_rewrites_out_until_it_exits() {
        let dir = tempfile::tempdir().unwrap();

        let app = AppLock::shared(dir.path()).unwrap();
        // A second app window or instance shares it.
        let second_app = AppLock::shared(dir.path()).unwrap();
        assert!(AppLock::try_exclusive(dir.path()).unwrap().is_none());

        drop(app);
        assert!(AppLock::try_exclusive(dir.path()).unwrap().is_none());
        drop(second_app);

        let rewrite = AppLock::try_exclusive(dir.path()).unwrap();
        assert!(rewrite.is_some());
        // Two rewrites never run at once either.
        assert!(AppLock::try_exclusive(dir.path()).unwrap().is_none());
    }

    #[test]
    fn the_lock_file_is_created_in_a_missing_cache_dir() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache").join("cerbo");

        assert!(AppLock::try_exclusive(&cache).unwrap().is_some());
        assert!(lock_path(&cache).is_file());
    }
}
