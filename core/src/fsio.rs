//! The single write path for every file Cerbo puts inside a vault.
//!
//! Everything goes through [`write_atomic`]: the bytes land in a temporary file
//! beside the destination, are flushed to storage, and are then renamed over the
//! destination. A reader therefore observes either the complete previous content
//! or the complete new content, never a partial state, and a write that fails
//! part-way leaves the destination exactly as it was.
//!
//! This module is the only place in `core` allowed to call `fs::write` or
//! `File::create`. `core/clippy.toml` denies both elsewhere, and
//! `scripts/check-atomic-writes.py` (wired in as the `atomic-writes` flake
//! check) fails the build on any other use.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Prefix on every temporary file [`write_atomic`] creates, so that a leftover
/// from an interrupted write is recognisable and can be skipped by vault scans.
pub const TEMP_PREFIX: &str = ".cerbo-tmp-";

/// True if `name` is a leftover temporary file from an interrupted [`write_atomic`].
pub fn is_temp_name(name: &str) -> bool {
    name.starts_with(TEMP_PREFIX)
}

/// Atomically replace `dest` with `bytes`, durably.
///
/// On success the contents are on storage and the new directory entry has been
/// flushed, so the write survives power loss. On failure `dest` still holds its
/// previous content and the temporary file is removed.
pub fn write_atomic(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = dest.parent().unwrap_or(Path::new("."));
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    // The temporary file must share the destination's directory: persist() uses
    // rename(2), which is only atomic within one filesystem.
    let mut tmp = tempfile::Builder::new()
        .prefix(TEMP_PREFIX)
        .tempfile_in(parent)?;

    tmp.write_all(bytes)?;
    tmp.flush()?;

    // persist() replaces the inode, so carry the destination's mode across.
    preserve_mode(dest, tmp.as_file())?;

    // tempfile synchronises neither the contents nor the containing directory,
    // so both fsyncs are ours to make.
    tmp.as_file().sync_data()?;

    tmp.persist(dest).map_err(|e| e.error)?;

    sync_dir(parent)?;

    #[cfg(test)]
    audit::record(dest);

    Ok(())
}

/// Test-only record of which destinations were actually replaced.
///
/// Lets a test assert *how many times* a file was written during one operation
/// — the difference between one backreference diff per save and two is
/// invisible from the final bytes alone.
#[cfg(test)]
pub mod audit {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    thread_local! {
        static WRITES: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record(dest: &Path) {
        WRITES.with(|w| w.borrow_mut().push(dest.to_path_buf()));
    }

    /// Run `f` and return every path `write_atomic` replaced while it ran.
    pub fn recording<T>(f: impl FnOnce() -> T) -> (T, Vec<PathBuf>) {
        WRITES.with(|w| w.borrow_mut().clear());
        let out = f();
        (out, WRITES.with(|w| w.borrow().clone()))
    }
}

/// [`write_atomic`] for text content.
pub fn write_atomic_str(dest: &Path, content: &str) -> io::Result<()> {
    write_atomic(dest, content.as_bytes())
}

#[cfg(unix)]
fn preserve_mode(dest: &Path, tmp: &File) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    match fs::metadata(dest) {
        Ok(meta) => tmp.set_permissions(fs::Permissions::from_mode(meta.permissions().mode())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(unix))]
fn preserve_mode(_dest: &Path, _tmp: &File) -> io::Result<()> {
    Ok(())
}

/// Flush the directory entry created by the rename, so the new name — not just
/// the new content — survives power loss.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn round_trips_bytes() {
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("page.md");

        write_atomic(&dest, b"# Title\n\nbody\n").unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"# Title\n\nbody\n");

        // Rewriting replaces rather than appends.
        write_atomic(&dest, b"replaced").unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"replaced");
    }

    #[test]
    fn leaves_no_temp_file_behind() {
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("page.md");
        write_atomic(&dest, b"x").unwrap();

        let leftovers: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .flatten()
            .filter(|e| is_temp_name(&e.file_name().to_string_lossy()))
            .collect();
        assert!(leftovers.is_empty(), "temp files left behind: {leftovers:?}");
    }

    #[test]
    #[cfg(unix)]
    fn preserves_destination_mode() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("secret.ttl");

        write_atomic(&dest, b"before").unwrap();
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o600)).unwrap();

        write_atomic(&dest, b"after").unwrap();

        let mode = fs::metadata(&dest).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "mode changed across an atomic rewrite");
        assert_eq!(fs::read(&dest).unwrap(), b"after");
    }

    #[test]
    #[cfg(unix)]
    fn failed_write_leaves_destination_intact() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("objects");
        fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("page.md");
        write_atomic(&dest, b"original content").unwrap();

        // An unwritable directory makes the temp file impossible to create, which
        // is the earliest point at which a real write can fail.
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).unwrap();
        let result = write_atomic(&dest, b"replacement that must never land");
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();

        if result.is_ok() {
            // Running as root defeats the mode bits; there is nothing to prove here.
            return;
        }

        assert!(result.is_err());
        assert_eq!(fs::read(&dest).unwrap(), b"original content");
    }
}
