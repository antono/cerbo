//! File fingerprints for the catch-up step (design D12).
//!
//! For every indexed object the table keeps `(mtime_ns, size)` of `meta.ttl`,
//! `annotations.ttl` and `page.md` as they were when the object was last
//! indexed. Comparing it with a fresh `stat` of the vault finds the objects
//! that git, file sync or an editor changed behind Cerbo's back.

use super::SemanticError;
use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// The files of an object that feed its graph, in fingerprint order.
pub const INDEXED_FILES: [&str; 3] = ["meta.ttl", "annotations.ttl", "page.md"];

/// Threads that `stat` a large vault in parallel.
const SCAN_THREADS: usize = 8;
/// Below this many objects one thread is faster than starting the pool.
const PARALLEL_THRESHOLD: usize = 1024;

/// Header of the binary table file, with its format version.
const MAGIC: &[u8; 4] = b"CFP1";

/// `(mtime in ns since the epoch, size)` of one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStamp {
    pub mtime_ns: i64,
    pub size: u64,
}

/// The stamps of [`INDEXED_FILES`]; `None` for a file that does not exist.
pub type ObjectStamp = [Option<FileStamp>; 3];

/// UUID → stamp of every object in the index.
pub type Fingerprints = HashMap<String, ObjectStamp>;

/// `stat` the indexed files of the object directory `dir`.
pub fn stamp_object(dir: &Path) -> io::Result<ObjectStamp> {
    let mut stamp = [None; 3];
    for (slot, name) in stamp.iter_mut().zip(INDEXED_FILES) {
        *slot = match std::fs::metadata(dir.join(name)) {
            Ok(meta) => Some(FileStamp {
                mtime_ns: meta
                    .modified()?
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as i64)
                    .unwrap_or(0),
                size: meta.len(),
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
    }
    Ok(stamp)
}

/// UUIDs of the live objects under `objects_dir`: directories named by a UUID.
/// The trash lives outside `objects/`, so trashed objects never appear.
pub fn list_objects(objects_dir: &Path) -> Result<Vec<String>, SemanticError> {
    let entries = match std::fs::read_dir(objects_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_error(objects_dir, e)),
    };
    let mut uuids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| io_error(objects_dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if uuid::Uuid::parse_str(&name).is_ok() && entry.file_type().is_ok_and(|t| t.is_dir()) {
            uuids.push(name);
        }
    }
    Ok(uuids)
}

/// Fingerprint every live object, on [`SCAN_THREADS`] threads for a large
/// vault. An object removed while the scan runs is simply left out.
pub fn scan(objects_dir: &Path) -> Result<Fingerprints, SemanticError> {
    let uuids = list_objects(objects_dir)?;
    let stamp_all = |chunk: &[String]| -> Result<Vec<(String, ObjectStamp)>, SemanticError> {
        let mut out = Vec::with_capacity(chunk.len());
        for uuid in chunk {
            let dir = objects_dir.join(uuid);
            match stamp_object(&dir) {
                Ok(stamp) if dir.is_dir() => out.push((uuid.clone(), stamp)),
                Ok(_) => {}
                Err(e) => return Err(io_error(&dir, e)),
            }
        }
        Ok(out)
    };

    if uuids.len() < PARALLEL_THRESHOLD {
        return Ok(stamp_all(&uuids)?.into_iter().collect());
    }
    let chunk_size = uuids.len().div_ceil(SCAN_THREADS);
    std::thread::scope(|scope| {
        let workers: Vec<_> = uuids
            .chunks(chunk_size)
            .map(|chunk| scope.spawn(move || stamp_all(chunk)))
            .collect();
        let mut table = Fingerprints::with_capacity(uuids.len());
        for worker in workers {
            table.extend(worker.join().expect("stat worker panicked")?);
        }
        Ok(table)
    })
}

/// Objects that are new or changed in `current`, and objects of `indexed`
/// that are gone. Both sorted, so updates are applied in a stable order.
pub fn diff(indexed: &Fingerprints, current: &Fingerprints) -> (Vec<String>, Vec<String>) {
    let mut changed: Vec<String> = current
        .iter()
        .filter(|(uuid, stamp)| indexed.get(*uuid) != Some(stamp))
        .map(|(uuid, _)| uuid.clone())
        .collect();
    let mut removed: Vec<String> = indexed
        .keys()
        .filter(|uuid| !current.contains_key(*uuid))
        .cloned()
        .collect();
    changed.sort();
    removed.sort();
    (changed, removed)
}

/// Read a table written by [`save`]. A missing file is `None`; so is a file in
/// another format, which only costs a full re-index.
pub fn load(path: &Path) -> Result<Option<Fingerprints>, SemanticError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_error(path, e)),
    };
    Ok(decode(&bytes))
}

/// Replace the table at `path` atomically.
pub fn save(path: &Path, table: &Fingerprints) -> Result<(), SemanticError> {
    crate::fsio::write_atomic(path, &encode(table)).map_err(|e| io_error(path, e))
}

fn encode(table: &Fingerprints) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + table.len() * 90);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(table.len() as u64).to_le_bytes());
    for (uuid, stamp) in table {
        out.push(uuid.len() as u8);
        out.extend_from_slice(uuid.as_bytes());
        for file in stamp {
            match file {
                Some(f) => {
                    out.push(1);
                    out.extend_from_slice(&f.mtime_ns.to_le_bytes());
                    out.extend_from_slice(&f.size.to_le_bytes());
                }
                None => out.push(0),
            }
        }
    }
    out
}

fn decode(bytes: &[u8]) -> Option<Fingerprints> {
    let mut r = Reader(bytes.strip_prefix(MAGIC.as_slice())?);
    let count = r.u64()? as usize;
    let mut table = Fingerprints::with_capacity(count.min(1 << 20));
    for _ in 0..count {
        let len = r.take(1)?[0] as usize;
        let uuid = std::str::from_utf8(r.take(len)?).ok()?.to_string();
        let mut stamp = [None; 3];
        for slot in &mut stamp {
            if r.take(1)?[0] == 1 {
                *slot = Some(FileStamp { mtime_ns: r.u64()? as i64, size: r.u64()? });
            }
        }
        table.insert(uuid, stamp);
    }
    Some(table)
}

/// Cursor over the table bytes; every read fails cleanly on a short file.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (head, tail) = self.0.split_at_checked(n)?;
        self.0 = tail;
        Some(head)
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
}

fn io_error(path: &Path, e: io::Error) -> SemanticError {
    SemanticError::Io(format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const A: &str = "a1b2c3d4-0000-4000-8000-000000000137";
    const B: &str = "b1b2c3d4-0000-4000-8000-000000000137";

    #[test]
    fn table_round_trips_through_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("fingerprints-1.bin");
        let mut table = Fingerprints::new();
        table.insert(A.into(), [Some(FileStamp { mtime_ns: 137, size: 1 }), None, None]);
        table.insert(B.into(), [None, None, Some(FileStamp { mtime_ns: -1, size: 137 })]);

        save(&path, &table).unwrap();
        assert_eq!(load(&path).unwrap(), Some(table));
        assert_eq!(load(&tmp.path().join("missing.bin")).unwrap(), None);

        fs::write(&path, b"garbage").unwrap();
        assert_eq!(load(&path).unwrap(), None, "a foreign file reads as no table");
    }

    #[test]
    fn scan_skips_non_objects_and_diff_finds_every_change() {
        let tmp = tempfile::tempdir().unwrap();
        let objects = tmp.path();
        for uuid in [A, B] {
            fs::create_dir_all(objects.join(uuid)).unwrap();
            fs::write(objects.join(uuid).join("page.md"), "x").unwrap();
        }
        fs::create_dir_all(objects.join("not-a-uuid")).unwrap();
        fs::write(objects.join(".cerbo-tmp-137"), "x").unwrap();

        let before = scan(objects).unwrap();
        assert_eq!(before.len(), 2);
        assert_eq!(diff(&before, &before), (vec![], vec![]));

        fs::write(objects.join(A).join("page.md"), "longer").unwrap();
        fs::remove_dir_all(objects.join(B)).unwrap();
        let after = scan(objects).unwrap();

        assert_eq!(diff(&before, &after), (vec![A.to_string()], vec![B.to_string()]));
        assert_eq!(diff(&Fingerprints::new(), &after), (vec![A.to_string()], vec![]));
    }
}
