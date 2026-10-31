//! The index's advisory `LOCK` (design D3).
//!
//! One process at a time holds the index, and only for one operation. The lock
//! is `std::fs::File::lock`'s `flock`, which also refuses a second handle in
//! the same process, so desktop threads queue up exactly like the CLI does.

use super::SemanticError;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

/// First pause between lock attempts; it doubles up to [`MAX_BACKOFF`].
const FIRST_BACKOFF: Duration = Duration::from_millis(10);
/// Longest pause between two lock attempts.
const MAX_BACKOFF: Duration = Duration::from_millis(250);

/// A held index lock. Dropping it (or the process dying) releases it.
#[derive(Debug)]
pub struct IndexLock {
    _file: File,
}

impl IndexLock {
    /// Take the lock at `path`, retrying with backoff for up to `timeout`, then
    /// fail with [`SemanticError::IndexBusy`].
    pub fn acquire(path: &Path, timeout: Duration) -> Result<Self, SemanticError> {
        // Opening never truncates: the file is only a lock handle, never data.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|e| SemanticError::Io(format!("cannot open {}: {e}", path.display())))?;

        let deadline = Instant::now() + timeout;
        let mut backoff = FIRST_BACKOFF;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self { _file: file }),
                Err(TryLockError::WouldBlock) => {}
                Err(TryLockError::Error(e)) => {
                    return Err(SemanticError::Io(format!("cannot lock {}: {e}", path.display())));
                }
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(SemanticError::IndexBusy);
            }
            thread::sleep(backoff.min(deadline - now));
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn held_lock_makes_another_thread_report_busy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("LOCK");

        let (held_tx, held_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let holder_path = path.clone();
        let holder = thread::spawn(move || {
            let _lock = IndexLock::acquire(&holder_path, Duration::ZERO).unwrap();
            held_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        held_rx.recv().unwrap();

        let timeout = Duration::from_millis(137);
        let started = Instant::now();
        let err = IndexLock::acquire(&path, timeout).unwrap_err();
        assert_eq!(err, SemanticError::IndexBusy);
        assert!(started.elapsed() >= timeout, "gave up before the timeout");

        release_tx.send(()).unwrap();
        holder.join().unwrap();
        IndexLock::acquire(&path, Duration::ZERO).expect("released lock is free");
    }

    #[test]
    fn waiter_gets_the_lock_once_it_is_released() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("LOCK");
        let lock = IndexLock::acquire(&path, Duration::ZERO).unwrap();

        let waiter_path = path.clone();
        let waiter =
            thread::spawn(move || IndexLock::acquire(&waiter_path, Duration::from_secs(5)).map(drop));
        thread::sleep(Duration::from_millis(50));
        drop(lock);

        waiter.join().unwrap().expect("waiter must get the released lock");
    }
}
