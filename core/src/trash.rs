//! The vault trash: what `object_delete` moved into `.cerbo/trash/`, listed and
//! put back.
//!
//! Entries are named `<timestamp>-<uuid>` (a suffix `-<n>` when the same object
//! went twice in one instant), see `object::object_delete`. Nothing here
//! removes anything from the trash: emptying it stays the user's call.

use crate::object::{object_path, trash_dir, ObjectMeta};
use crate::CerboContext;
use serde_json::{json, Value};
use std::path::PathBuf;

/// One trash entry on disk.
struct Entry {
    path: PathBuf,
    /// The entry's name, which sorts by deletion time.
    name: String,
    uuid: String,
}

/// `<timestamp>-<uuid>[-n]` into the object's UUID; other names are not ours.
fn uuid_of(name: &str) -> Option<String> {
    let (_stamp, rest) = name.split_once('-')?;
    let candidate = rest.get(..36)?;
    uuid::Uuid::parse_str(candidate).ok()?;
    let tail = &rest[36..];
    (tail.is_empty() || tail.strip_prefix('-').is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()))).then(|| candidate.to_string())
}

fn entries(ctx: &CerboContext) -> Result<Vec<Entry>, String> {
    let dir = trash_dir(ctx);
    let read = match std::fs::read_dir(&dir) {
        Ok(read) => read,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", dir.display())),
    };
    let mut out: Vec<Entry> = read
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            uuid_of(&name).map(|uuid| Entry { path: e.path(), name, uuid })
        })
        .collect();
    // Newest first: the timestamp leads the name.
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

fn title_of(entry: &Entry) -> String {
    ObjectMeta::read_from_file(&entry.path.join("meta.ttl")).map(|m| m.title).unwrap_or_default()
}

/// The trash, newest first, as `{uuid, title, deleted}`; `deleted` is the
/// timestamp in the entry's name (UTC, `YYYYMMDDTHHMMSS…Z`).
pub fn list(ctx: &CerboContext) -> Result<Value, String> {
    let rows: Vec<Value> = entries(ctx)?
        .iter()
        .map(|e| {
            json!({
                "uuid": e.uuid,
                "title": title_of(e),
                "deleted": e.name.split_once('-').map(|(stamp, _)| stamp).unwrap_or_default(),
            })
        })
        .collect();
    Ok(Value::Array(rows))
}

/// Put the newest trash entry of `uuid` back. Refuses when an object with that
/// UUID is live again, and when the trash holds none.
pub fn restore(ctx: &CerboContext, uuid: &str) -> Result<Value, String> {
    uuid::Uuid::parse_str(uuid).map_err(|_| format!("not a page UUID: {uuid}"))?;
    let target = object_path(ctx, uuid);
    if target.exists() {
        return Err(format!("{uuid} already exists in the vault; nothing was restored"));
    }
    let entry = entries(ctx)?
        .into_iter()
        .find(|e| e.uuid == uuid)
        .ok_or_else(|| format!("the trash holds nothing for {uuid}"))?;
    let title = title_of(&entry);
    std::fs::rename(&entry.path, &target).map_err(|e| format!("cannot restore {uuid}: {e}"))?;
    crate::object::sync_index(ctx, uuid);
    Ok(json!({ "uuid": uuid, "title": title }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> (tempfile::TempDir, CerboContext) {
        let tmp = tempfile::tempdir().unwrap();
        let cerbo = tmp.path().join(".cerbo");
        std::fs::create_dir_all(cerbo.join("objects")).unwrap();
        let ctx = CerboContext { config_dir: cerbo, cache_dir: tmp.path().join("cache") };
        (tmp, ctx)
    }

    #[test]
    fn a_deleted_page_is_listed_and_can_be_restored() {
        let (_tmp, ctx) = ctx();
        let id = crate::page::page_create(&ctx, "Old idea".to_string()).unwrap();
        crate::page::page_write(&ctx, id.clone(), "# Old idea\n\ntext\n".to_string()).unwrap();
        crate::object::object_delete(&ctx, &id).unwrap();

        let listed = list(&ctx).unwrap();
        assert_eq!(listed[0]["uuid"], id.as_str());
        assert_eq!(listed[0]["title"], "Old idea");
        assert!(listed[0]["deleted"].as_str().unwrap().ends_with('Z'));
        assert!(!object_path(&ctx, &id).exists());

        let back = restore(&ctx, &id).unwrap();
        assert_eq!(back["title"], "Old idea");
        assert_eq!(std::fs::read_to_string(object_path(&ctx, &id).join("page.md")).unwrap(), "# Old idea\n\ntext\n");
        assert!(list(&ctx).unwrap().as_array().unwrap().is_empty());
    }

    #[test]
    fn the_newest_entry_wins_and_a_live_page_is_never_overwritten() {
        let (_tmp, ctx) = ctx();
        let id = crate::page::page_create(&ctx, "Twice".to_string()).unwrap();
        crate::page::page_write(&ctx, id.clone(), "newer\n".to_string()).unwrap();
        crate::object::object_delete(&ctx, &id).unwrap();
        // An older entry of the same object, as if it had been deleted before.
        let newer = entries(&ctx).unwrap().remove(0).path;
        let older = trash_dir(&ctx).join(format!("20200101T000000.000000Z-{id}"));
        std::fs::create_dir_all(&older).unwrap();
        for file in ["meta.ttl", "page.md"] {
            std::fs::copy(newer.join(file), older.join(file)).unwrap();
        }
        std::fs::write(older.join("page.md"), "older\n").unwrap();
        assert_eq!(list(&ctx).unwrap().as_array().unwrap().len(), 2);

        restore(&ctx, &id).unwrap();
        assert_eq!(std::fs::read_to_string(object_path(&ctx, &id).join("page.md")).unwrap(), "newer\n");
        // The older entry stays in the trash, and the page is live: refused.
        assert_eq!(list(&ctx).unwrap().as_array().unwrap().len(), 1);
        let err = restore(&ctx, &id).unwrap_err();
        assert!(err.contains("already exists"), "{err}");
        assert_eq!(std::fs::read_to_string(object_path(&ctx, &id).join("page.md")).unwrap(), "newer\n");
    }

    #[test]
    fn nothing_to_restore_and_bad_ids_are_errors() {
        let (_tmp, ctx) = ctx();
        assert!(list(&ctx).unwrap().as_array().unwrap().is_empty());
        let none = restore(&ctx, "00000000-0000-4000-8000-000000000137").unwrap_err();
        assert!(none.contains("nothing"), "{none}");
        assert!(restore(&ctx, "../etc").is_err());
    }

    #[test]
    fn only_trash_names_are_read() {
        assert_eq!(
            uuid_of("20261120T101010.123456Z-00000000-0000-4000-8000-000000000137").as_deref(),
            Some("00000000-0000-4000-8000-000000000137")
        );
        assert!(uuid_of("20261120T101010.123456Z-00000000-0000-4000-8000-000000000137-2").is_some());
        assert!(uuid_of("20261120T101010.123456Z-00000000-0000-4000-8000-000000000137-x").is_none());
        assert!(uuid_of("notes").is_none());
    }
}
