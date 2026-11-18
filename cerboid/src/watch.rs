//! File watching and change events (spec "Change events").
//!
//! One watcher per vault reports every change under `.cerbo/`, whoever made
//! it. Events are debounced per object and labelled with their origin: a page
//! the daemon just wrote for a client carries that client's kind, anything
//! else is an outside edit.

use cerbo_core::rpc::{ChangeKind, Event, Origin};
use notify::{RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

/// How long after a write the daemon still attributes file events to it.
const ATTRIBUTION_TTL: Duration = Duration::from_secs(3);

/// Who the daemon wrote for recently, so the file events its own writes cause
/// are not reported as outside edits.
#[derive(Default)]
pub struct WriteTracker {
    recent: Mutex<HashMap<(String, String), (Origin, Instant)>>,
}

impl WriteTracker {
    pub fn note(&self, vault: &str, page: &str, origin: Origin) {
        let mut recent = self.recent.lock().unwrap_or_else(|e| e.into_inner());
        recent.retain(|_, (_, at)| at.elapsed() < ATTRIBUTION_TTL);
        recent.insert((vault.to_string(), page.to_string()), (origin, Instant::now()));
    }

    /// The origin of a recent daemon write to `page`, if there was one.
    pub fn origin(&self, vault: &str, page: &str) -> Option<Origin> {
        let recent = self.recent.lock().unwrap_or_else(|e| e.into_inner());
        recent
            .get(&(vault.to_string(), page.to_string()))
            .filter(|(_, at)| at.elapsed() < ATTRIBUTION_TTL)
            .map(|(origin, _)| *origin)
    }
}

/// The object a changed path belongs to: its first component under
/// `objects_dir`. Leftovers of atomic writes and anything that is not a UUID
/// are not objects.
pub fn object_of(objects_dir: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(objects_dir).ok()?;
    let mut parts = rel.components().filter_map(|c| match c {
        Component::Normal(name) => name.to_str(),
        _ => None,
    });
    let first = parts.next()?;
    if cerbo_core::fsio::is_temp_name(first) {
        return None;
    }
    // A temporary file inside the object is the half of an atomic write; the
    // rename that completes it is reported on its own.
    if parts.any(cerbo_core::fsio::is_temp_name) {
        return None;
    }
    uuid::Uuid::parse_str(first).ok().map(|u| u.hyphenated().to_string())
}

/// Watch `<root>/.cerbo/` and broadcast an [`Event::PageChanged`] for every
/// object that changed, once per `quiet` period. The watcher stops when the
/// returned value is dropped.
pub fn start(
    root: &Path,
    vault_id: String,
    events: broadcast::Sender<Event>,
    tracker: Arc<WriteTracker>,
    quiet: Duration,
) -> Result<notify::RecommendedWatcher, String> {
    let cerbo_dir = root.join(".cerbo");
    let objects_dir = cerbo_dir.join("objects");
    let (tx, rx) = mpsc::channel::<String>();

    let filter_dir = objects_dir.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(event) = res else { return };
        if matches!(event.kind, notify::EventKind::Access(_)) {
            return;
        }
        for path in &event.paths {
            if let Some(object) = object_of(&filter_dir, path) {
                let _ = tx.send(object);
            }
        }
    })
    .map_err(|e| format!("cannot watch {}: {e}", cerbo_dir.display()))?;
    watcher
        .watch(&cerbo_dir, RecursiveMode::Recursive)
        .map_err(|e| format!("cannot watch {}: {e}", cerbo_dir.display()))?;

    // Ends when the watcher (and with it `tx`) is dropped.
    std::thread::spawn(move || {
        while let Ok(first) = rx.recv() {
            let mut changed = HashSet::from([first]);
            // Collect until the vault is quiet for `quiet`.
            while let Ok(more) = rx.recv_timeout(quiet) {
                changed.insert(more);
            }
            for page in changed {
                let change =
                    if objects_dir.join(&page).is_dir() { ChangeKind::Changed } else { ChangeKind::Removed };
                let origin = tracker.origin(&vault_id, &page).unwrap_or(Origin::External);
                // No subscriber is not an error.
                let _ = events.send(Event::PageChanged { vault: vault_id.clone(), page, change, origin });
            }
        }
    });
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const ID: &str = "1e5d7dc7-a34c-4a2a-aa49-8bfe2b9805b6";

    #[test]
    fn a_file_of_an_object_maps_to_the_object() {
        let objects = PathBuf::from("/v/.cerbo/objects");
        let page = objects.join(ID).join("page.md");
        assert_eq!(object_of(&objects, &page).as_deref(), Some(ID));
        assert_eq!(object_of(&objects, &objects.join(ID)).as_deref(), Some(ID));
    }

    #[test]
    fn temp_files_and_strangers_are_not_objects() {
        let objects = PathBuf::from("/v/.cerbo/objects");
        let temp = objects.join(ID).join(".cerbo-tmp-abc");
        assert_eq!(object_of(&objects, &temp), None);
        assert_eq!(object_of(&objects, &objects.join(".cerbo-tmp-xyz").join("meta.ttl")), None);
        assert_eq!(object_of(&objects, &objects.join("not-a-uuid").join("page.md")), None);
        assert_eq!(object_of(&objects, &PathBuf::from("/v/.cerbo/trash").join(ID)), None);
        assert_eq!(object_of(&objects, &objects), None);
    }

    #[test]
    fn recent_writes_are_attributed_to_their_client() {
        let tracker = WriteTracker::default();
        assert_eq!(tracker.origin("v", ID), None);
        tracker.note("v", ID, Origin::Agent);
        assert_eq!(tracker.origin("v", ID), Some(Origin::Agent));
        assert_eq!(tracker.origin("other", ID), None);
    }
}
