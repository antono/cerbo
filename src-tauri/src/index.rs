use crate::get_vault_ctx;
use crate::semantic::IndexGate;
use cerbo_core::semantic::{BacklinkEntry, IndexOptions, SemanticError, SemanticIndex};
use cerbo_core::VaultContext;
use notify::{RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

/// Quiet time after the last file event before the index catches up, so one
/// save (several files, temp file plus rename) becomes one index update.
const DEBOUNCE: Duration = Duration::from_millis(300);
/// Pause before retrying a catch-up that failed, e.g. while the CLI holds the
/// index.
const RETRY_DELAY: Duration = Duration::from_secs(2);
/// Emitted whenever what the backlinks panel should show may have changed:
/// the opened vault's index is built and caught up, its first catch-up failed
/// (the panel then shows why instead of loading forever), or an update
/// succeeds after a failed one (e.g. the CLI held the index). A panel that
/// shows a loading, busy or error state loads again.
pub const INDEX_READY_EVENT: &str = "semantic-index-ready";

/// The active vault's file watcher, which keeps its semantic index current
/// (design D12, hybrid): one full catch-up when the vault opens, then only the
/// objects the watcher reports.
#[derive(Default)]
pub struct WatcherState {
    /// Replaced on every vault open; dropping it stops the old vault's worker.
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
    /// Set when watcher events were lost or a catch-up failed: the next index
    /// update must be a full scan rather than trust the reported paths. A new
    /// flag per vault open, so the old vault's worker cannot clear it.
    pub full_scan_needed: Mutex<Arc<AtomicBool>>,
    /// How the vault-open catch-up (a build, for a vault without an index)
    /// went. A new one per vault open, like the flag above.
    pub status: Mutex<Arc<Mutex<IndexStatus>>>,
}

/// The index worker's progress on the opened vault.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum IndexStatus {
    /// The vault-open catch-up has not finished yet.
    #[default]
    Building,
    /// The vault-open catch-up has failed every time so far, last with this
    /// error; the worker keeps retrying.
    Failed(SemanticError),
    /// An update has succeeded. Later failures are left to the next request,
    /// which catches up itself.
    Ready,
}

impl IndexStatus {
    /// The status after an update with `result`, and whether the panel should
    /// load again.
    fn after(&self, result: Result<(), &SemanticError>, recovered: bool) -> (Self, bool) {
        match (self, result) {
            (Self::Ready, Ok(())) => (Self::Ready, recovered),
            (_, Ok(())) => (Self::Ready, true),
            (Self::Ready, Err(_)) => (Self::Ready, false),
            // Only a different kind of failure is announced: a message that
            // merely varies between attempts (OS details, say) would make the
            // panel reload on every retry.
            (Self::Failed(old), Err(e)) => {
                (Self::Failed(e.clone()), std::mem::discriminant(old) != std::mem::discriminant(e))
            }
            (Self::Building, Err(e)) => (Self::Failed(e.clone()), true),
        }
    }
}

/// What the watcher tells the index worker.
enum IndexEvent {
    /// Files of this object changed.
    Object(String),
    /// Events were dropped or the watcher failed: scan everything.
    Rescan,
}


/// Why `backlinks_get` has no list; the panel shows each differently and the
/// page stays open and editable in every case.
#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BacklinksError {
    /// The opened vault's index is still being built: show a loading state
    /// and load again on [`INDEX_READY_EVENT`].
    Building,
    /// Another Cerbo process held the index longer than the retry limit.
    IndexBusy,
    /// Anything else, such as an index that cannot be opened.
    Unavailable { message: String },
}

impl From<SemanticError> for BacklinksError {
    fn from(e: SemanticError) -> Self {
        match e {
            SemanticError::IndexBusy => Self::IndexBusy,
            other => Self::Unavailable { message: other.to_string() },
        }
    }
}

impl From<String> for BacklinksError {
    fn from(message: String) -> Self {
        Self::Unavailable { message }
    }
}

/// The pages linking to `uuid`, from the vault's semantic index (design D11).
/// Runs off the main thread: opening the index may wait for its lock.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn backlinks_get(
    app: AppHandle,
    _vaultId: String,
    uuid: String,
) -> Result<Vec<BacklinkEntry>, BacklinksError> {
    tauri::async_runtime::spawn_blocking(move || backlinks(&app, &uuid))
        .await
        .map_err(|e| BacklinksError::Unavailable { message: e.to_string() })?
}

fn backlinks(app: &AppHandle, uuid: &str) -> Result<Vec<BacklinkEntry>, BacklinksError> {
    let ctx = get_vault_ctx(app)?;
    let watcher_state = app.state::<WatcherState>();
    let poisoned = || BacklinksError::from("watcher state lock poisoned".to_string());
    let status = Arc::clone(&*watcher_state.status.lock().map_err(|_| poisoned())?);
    let status = status.lock().map_err(|_| poisoned())?.clone();
    match status {
        IndexStatus::Building => return Err(BacklinksError::Building),
        // Busy shows as "temporarily unavailable", anything else as an error.
        IndexStatus::Failed(e) => return Err(e.into()),
        IndexStatus::Ready => {}
    }
    let full_scan_needed = Arc::clone(&*watcher_state.full_scan_needed.lock().map_err(|_| poisoned())?);

    let vault_path = ctx.config_dir.parent().unwrap_or(&ctx.config_dir).to_path_buf();
    let vault_ctx = VaultContext { vault_path, global: crate::get_context(app)? };
    // Behind this process's other index work, e.g. "Enable" (see IndexGate).
    let gate = app.state::<IndexGate>();
    let _turn = gate.hold();
    let mut index = SemanticIndex::open(&vault_ctx, IndexOptions::default())?;
    // The watcher keeps the index current; only after lost events or a failed
    // update does it need a full scan first (design D12).
    if index.state().stale || full_scan_needed.swap(false, Ordering::SeqCst) {
        if let Err(e) = index.catch_up(None) {
            full_scan_needed.store(true, Ordering::SeqCst);
            return Err(e.into());
        }
    }
    // Object directories are named by the lowercase UUID the index uses.
    Ok(index.backlinks(&uuid.to_ascii_lowercase())?)
}

pub fn start_watcher(
    app: &AppHandle,
    _vault_id: &str,
    vault_path: PathBuf,
    watcher_state: &State<WatcherState>,
) -> Result<(), String> {
    let vault_ctx = VaultContext { vault_path, global: crate::get_context(app)? };
    let objects_dir = vault_ctx.objects_dir();
    // A new or empty vault may have no objects directory yet; the first page
    // would create it anyway, and the watcher needs it to exist.
    std::fs::create_dir_all(&objects_dir)
        .map_err(|e| format!("Failed to create {}: {e}", objects_dir.display()))?;
    // Saves never wait for the index, and one that finds it busy leaves the
    // object to the worker below, which hears of it from the watcher; marking
    // the index stale instead would make the worker rebuild it after every
    // such save.
    cerbo_core::semantic::set_sync_watched(true);
    let (tx, rx) = mpsc::channel();

    let watched = objects_dir.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let events: Vec<IndexEvent> = match res {
            Ok(event) if event.need_rescan() => vec![IndexEvent::Rescan],
            Ok(event) => event
                .paths
                .iter()
                .filter_map(|p| object_uuid(&watched, p))
                .map(IndexEvent::Object)
                .collect(),
            Err(e) => {
                eprintln!("Warning: file watcher error, the index will rescan: {e}");
                vec![IndexEvent::Rescan]
            }
        };
        for event in events {
            // The worker is gone only when this watcher is being replaced.
            let _ = tx.send(event);
        }
    })
    .map_err(|e| format!("Failed to start file watcher: {e}"))?;
    watcher
        .watch(&objects_dir, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch {}: {e}", objects_dir.display()))?;

    // A new vault starts with a full catch-up.
    let full_scan_needed = Arc::new(AtomicBool::new(true));
    let status = Arc::new(Mutex::new(IndexStatus::Building));
    let worker_flag = Arc::clone(&full_scan_needed);
    let worker_status = Arc::clone(&status);
    let worker_app = app.clone();
    std::thread::spawn(move || index_worker(worker_app, vault_ctx, rx, worker_flag, worker_status));

    let poisoned = "watcher state lock poisoned".to_string();
    *watcher_state.full_scan_needed.lock().map_err(|_| poisoned.clone())? = full_scan_needed;
    *watcher_state.status.lock().map_err(|_| poisoned.clone())? = status;
    *watcher_state.watcher.lock().map_err(|_| poisoned)? = Some(watcher);
    Ok(())
}

/// The name of the entry directly under `objects_dir` that `path` is in: the
/// object's UUID. Other names (temp files) are ignored by `catch_up_paths`.
fn object_uuid(objects_dir: &Path, path: &Path) -> Option<String> {
    let first = path.strip_prefix(objects_dir).ok()?.components().next()?;
    first.as_os_str().to_str().map(str::to_string)
}

/// Apply watcher events to the index until the watcher is dropped. Events are
/// debounced and batched; a full scan runs first, after lost events, and after
/// any failed update (retried every [`RETRY_DELAY`]). Each result moves the
/// `status` on (see [`IndexStatus::after`]), emitting [`INDEX_READY_EVENT`]
/// when the panel should load again.
fn index_worker(
    app: AppHandle,
    vault_ctx: VaultContext,
    rx: mpsc::Receiver<IndexEvent>,
    full_scan_needed: Arc<AtomicBool>,
    status: Arc<Mutex<IndexStatus>>,
) {
    let mut pending: Vec<String> = Vec::new();
    let mut failed = false;
    loop {
        if !full_scan_needed.load(Ordering::SeqCst) && pending.is_empty() {
            match rx.recv() {
                Ok(event) => collect(event, &mut pending, &full_scan_needed),
                Err(_) => return,
            }
        }
        // Debounce: keep collecting until the vault is quiet.
        loop {
            match rx.recv_timeout(DEBOUNCE) {
                Ok(event) => collect(event, &mut pending, &full_scan_needed),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }

        let full = full_scan_needed.swap(false, Ordering::SeqCst);
        let gate = app.state::<IndexGate>();
        let turn = gate.hold();
        let result = SemanticIndex::open(&vault_ctx, IndexOptions::default()).and_then(|mut index| {
            if full {
                index.catch_up(None)
            } else {
                index.catch_up_paths(&pending)
            }
        });
        drop(turn);
        let recovered = result.is_ok() && std::mem::take(&mut failed);
        let announce = {
            let mut status = status.lock().unwrap_or_else(PoisonError::into_inner);
            let (next, announce) = status.after(result.as_ref().map(drop), recovered);
            *status = next;
            announce
        };
        if announce {
            if let Err(e) = app.emit(INDEX_READY_EVENT, ()) {
                eprintln!("Warning: could not announce the index status: {e}");
            }
        }
        pending.clear();
        if let Err(e) = result {
            eprintln!("Warning: index update failed, retrying with a full scan: {e}");
            full_scan_needed.store(true, Ordering::SeqCst);
            failed = true;
            std::thread::sleep(RETRY_DELAY);
        }
    }
}

fn collect(event: IndexEvent, pending: &mut Vec<String>, full_scan_needed: &AtomicBool) {
    match event {
        IndexEvent::Object(uuid) => {
            if !pending.contains(&uuid) {
                pending.push(uuid);
            }
        }
        IndexEvent::Rescan => full_scan_needed.store(true, Ordering::SeqCst),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_uuid_names_the_object_directory() {
        let objects = Path::new("/vault/.cerbo/objects");
        let uuid = "a1b2c3d4-0000-4000-8000-000000000137";
        assert_eq!(object_uuid(objects, &objects.join(uuid).join("page.md")), Some(uuid.to_string()));
        assert_eq!(object_uuid(objects, &objects.join(uuid)), Some(uuid.to_string()));
        assert_eq!(object_uuid(objects, Path::new("/vault/.cerbo/trash/x")), None);
    }

    #[test]
    fn a_failing_vault_open_catch_up_is_shown_not_loaded_forever() {
        let busy = SemanticError::IndexBusy;
        let broken = SemanticError::Io("index storage: 137".into());

        // The first attempt fails: the panel stops loading and shows why.
        let (status, announce) = IndexStatus::Building.after(Err(&busy), false);
        assert_eq!(status, IndexStatus::Failed(busy.clone()));
        assert!(announce);
        assert!(matches!(BacklinksError::from(busy.clone()), BacklinksError::IndexBusy));

        // The same failure again changes nothing; another one is shown.
        assert_eq!(status.after(Err(&busy), false), (IndexStatus::Failed(busy.clone()), false));
        let (status, announce) = status.after(Err(&broken), false);
        assert_eq!(status, IndexStatus::Failed(broken.clone()));
        assert!(announce);
        assert!(matches!(BacklinksError::from(broken.clone()), BacklinksError::Unavailable { .. }));

        // Success at last: the panel loads the list.
        let (status, announce) = status.after(Ok(()), true);
        assert_eq!(status, IndexStatus::Ready);
        assert!(announce);
    }

    #[test]
    fn a_failure_whose_message_varies_is_not_announced_again() {
        let first = SemanticError::Io("index storage: 137".into());
        let again = SemanticError::Io("index storage: 138".into());
        let (status, announce) = IndexStatus::Failed(first).after(Err(&again), false);
        assert_eq!(status, IndexStatus::Failed(again));
        assert!(!announce);
    }

    #[test]
    fn a_ready_index_announces_only_its_first_success_and_recoveries() {
        assert_eq!(IndexStatus::Building.after(Ok(()), false), (IndexStatus::Ready, true));
        assert_eq!(IndexStatus::Ready.after(Ok(()), false), (IndexStatus::Ready, false));
        let busy = SemanticError::IndexBusy;
        assert_eq!(IndexStatus::Ready.after(Err(&busy), false), (IndexStatus::Ready, false));
        assert_eq!(IndexStatus::Ready.after(Ok(()), true), (IndexStatus::Ready, true));
    }
}
