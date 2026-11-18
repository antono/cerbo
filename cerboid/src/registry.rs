//! The vaults the daemon serves (spec "Daemon serves every registered vault").
//!
//! The list is the user's vault list (`vaults.toml` and the auto-registered
//! vaults). It is re-read when its files change, so `cerbo vault add` and
//! `vault remove` take effect without a restart.

use crate::queue::WriteQueue;
use crate::watch::{self, WriteTracker};
use cerbo_core::rpc::{Event, VaultInfo};
use cerbo_core::{CerboContext, VaultContext};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::sync::broadcast;

/// One served vault: its paths, its write queue and its watcher.
pub struct VaultHandle {
    pub info: VaultInfo,
    pub root: PathBuf,
    /// The context object and page functions take: `config_dir` is the
    /// vault's `.cerbo/`.
    pub page_ctx: CerboContext,
    pub vault_ctx: VaultContext,
    pub queue: WriteQueue,
    /// Serialises the daemon's own use of the semantic index, so its readers
    /// and its writer never fight over the index lock.
    pub index_gate: Mutex<()>,
    _watcher: Option<notify::RecommendedWatcher>,
}

impl VaultHandle {
    /// Hold the index gate for the duration of an index operation.
    pub fn index_guard(&self) -> std::sync::MutexGuard<'_, ()> {
        self.index_gate.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// How the registry watches its vaults.
#[derive(Clone)]
pub struct WatchSettings {
    pub events: broadcast::Sender<Event>,
    pub tracker: Arc<WriteTracker>,
    pub quiet: Duration,
}

type Stamp = [Option<(SystemTime, u64)>; 2];

struct Inner {
    vaults: Vec<Arc<VaultHandle>>,
    stamp: Stamp,
}

pub struct Registry {
    ctx: CerboContext,
    watch: WatchSettings,
    inner: Mutex<Inner>,
}

impl Registry {
    /// Load the vault list and start serving it.
    pub fn new(ctx: CerboContext, watch: WatchSettings) -> Result<Self, String> {
        let registry = Self { ctx, watch, inner: Mutex::new(Inner { vaults: Vec::new(), stamp: [None, None] }) };
        registry.refresh()?;
        Ok(registry)
    }

    /// Re-read the vault list. Returns whether the set of served vaults changed.
    pub fn refresh(&self) -> Result<bool, String> {
        let stamp = self.stamp();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if stamp == inner.stamp && !inner.vaults.is_empty() {
            return Ok(false);
        }
        let listed = cerbo_core::vault::vault_list(&self.ctx)?.vaults;
        inner.stamp = stamp;

        let mut next: Vec<Arc<VaultHandle>> = Vec::new();
        let mut changed = false;
        for vault in &listed {
            let kept = inner.vaults.iter().find(|h| h.info.id == vault.id && h.root == vault.path);
            match kept {
                Some(handle) => next.push(Arc::clone(handle)),
                None => {
                    // Skip a vault whose directory is gone; it comes back
                    // when the list is read again after it reappears.
                    if !vault.path.join(".cerbo").is_dir() {
                        eprintln!("cerboid: not serving vault {}: {} is not a vault", vault.name, vault.path.display());
                        continue;
                    }
                    next.push(Arc::new(self.open(vault)));
                    changed = true;
                }
            }
        }
        if next.len() != inner.vaults.len() {
            changed = true;
        }
        // Dropped handles stop their watcher; a queue keeps draining what it
        // already took because its worker owns the receiver.
        inner.vaults = next;
        if changed {
            // Announced here, not by the caller: a refresh triggered by a
            // lookup of a new vault must not swallow the event the poller
            // would otherwise have sent.
            let _ = self.watch.events.send(Event::VaultsChanged);
        }
        Ok(changed)
    }

    fn open(&self, vault: &cerbo_core::vault::Vault) -> VaultHandle {
        let root = vault.path.clone();
        let page_ctx = CerboContext { config_dir: root.join(".cerbo"), cache_dir: self.ctx.cache_dir.clone() };
        let vault_ctx = VaultContext { vault_path: root.clone(), global: self.ctx.clone() };
        let watcher = match watch::start(
            &root,
            vault.id.clone(),
            self.watch.events.clone(),
            Arc::clone(&self.watch.tracker),
            self.watch.quiet,
        ) {
            Ok(w) => Some(w),
            Err(e) => {
                eprintln!("cerboid: {e}; changes made outside Cerbo to {} will not be announced", vault.name);
                None
            }
        };
        VaultHandle {
            info: VaultInfo {
                id: vault.id.clone(),
                name: vault.name.clone(),
                path: root.to_string_lossy().into_owned(),
            },
            root,
            page_ctx,
            vault_ctx,
            queue: WriteQueue::new(),
            index_gate: Mutex::new(()),
            _watcher: watcher,
        }
    }

    fn stamp(&self) -> Stamp {
        let of = |path: Result<PathBuf, String>| {
            let meta = std::fs::metadata(path.ok()?).ok()?;
            Some((meta.modified().ok()?, meta.len()))
        };
        [
            of(Ok(self.ctx.config_dir.join("vaults.toml"))),
            of(cerbo_core::config::auto_config_path(&self.ctx)),
        ]
    }

    /// The vaults served now.
    pub fn list(&self) -> Vec<Arc<VaultHandle>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).vaults.clone()
    }

    /// Find a vault by id, name or path. A vault added a moment ago is found
    /// too: an unknown name triggers one re-read of the list.
    pub fn resolve(&self, selector: &str) -> Option<Arc<VaultHandle>> {
        if let Some(found) = self.find(selector) {
            return Some(found);
        }
        self.refresh().ok()?;
        self.find(selector)
    }

    fn find(&self, selector: &str) -> Option<Arc<VaultHandle>> {
        let vaults = self.list();
        let by = |pred: &dyn Fn(&VaultHandle) -> bool| vaults.iter().find(|v| pred(v)).cloned();
        by(&|v| v.info.id == selector)
            .or_else(|| by(&|v| v.info.name == selector))
            .or_else(|| {
                let wanted = canonical(Path::new(selector));
                by(&|v| canonical(&v.root) == wanted)
            })
    }

    /// Stop taking writes and wait until every queue has finished its jobs.
    pub async fn close_all(&self) {
        for vault in self.list() {
            vault.queue.close_and_drain().await;
        }
    }
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
