//! One operation on a vault's index: lock, open or build, catch up, answer,
//! release (design D3, D9, D12).

use super::embed::EmbedProgress;
use super::fingerprints::{self, Fingerprints};
use super::graph::{read_objects, ObjectGraph, OxigraphIndex};
use super::query::{query_prefixes, QueryOutput};
use super::{GraphIndex, IndexLocation, IndexLock, IndexState, SemanticError, LOCK_TIMEOUT};
use crate::prefix_map::PrefixMap;
use crate::VaultContext;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Name of the similarity graph under `cerbo://graphs/` (design D7).
const SIMILARITY_GRAPH: &str = "similarity";

/// A step of a full build, reported while it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildProgress {
    /// The build found this many live objects.
    Started { objects: usize },
    /// Objects read from the vault so far.
    Read { done: usize, total: usize },
    /// The new generation is complete on disk; only the switch to it remains.
    Loaded { triples: u64 },
    /// `state.json` now names the new generation.
    Finished { generation: u64, triples: u64 },
}

/// Receives [`BuildProgress`]; called from worker threads, hence `Sync`.
pub type Progress<'a> = Option<&'a (dyn Fn(BuildProgress) + Sync)>;

/// How to open an index.
#[derive(Clone, Copy)]
pub struct IndexOptions<'a> {
    /// How long to wait for another process to release the index.
    pub lock_timeout: Duration,
    /// Progress of a build, should one be needed.
    pub progress: Progress<'a>,
    /// Progress of an embedding refresh, as `(done, total)` pages.
    pub embed_progress: EmbedProgress<'a>,
}

impl Default for IndexOptions<'_> {
    fn default() -> Self {
        Self { lock_timeout: LOCK_TIMEOUT, progress: None, embed_progress: None }
    }
}

/// What a catch-up step changed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CatchUp {
    /// Objects whose graph was replaced because they are new or changed.
    pub upserted: usize,
    /// Objects whose graph was dropped because they are gone.
    pub removed: usize,
    /// The stale flag forced a full rebuild instead.
    pub rebuilt: bool,
}

/// A vault's index, open and locked for one operation. Dropping it closes the
/// store and then releases the lock.
pub struct SemanticIndex {
    pub(super) location: IndexLocation,
    pub(super) objects_dir: PathBuf,
    pub(super) cerbo_dir: PathBuf,
    pub(super) state: IndexState,
    pub(super) fingerprints: Fingerprints,
    graph: Option<OxigraphIndex>,
    /// `state` is this vault's live generation in the current format, so a
    /// rebuild may carry its embeddings over even when its store or its
    /// fingerprints could not be opened.
    live_state: bool,
    // Declared last: fields drop in order, so the store closes before unlock.
    _lock: IndexLock,
}

impl SemanticIndex {
    /// Lock the index of `vault_ctx` and open its live generation. An index
    /// that is missing, from another format version, or recorded for another
    /// vault path is built first (spec "Missing or incompatible index is built
    /// automatically").
    pub fn open(vault_ctx: &VaultContext, options: IndexOptions<'_>) -> Result<Self, SemanticError> {
        let location = IndexLocation::for_vault(&vault_ctx.global.cache_dir, &vault_ctx.vault_path)?;
        let index = Self::open_at(vault_ctx, location, options, true)?;
        Ok(index.expect("an index that may be built always opens"))
    }

    /// Like [`Self::open`], but never builds: `None` when the vault has no
    /// usable index yet. Nothing is created in the cache in that case.
    pub fn open_existing(vault_ctx: &VaultContext, options: IndexOptions<'_>) -> Result<Option<Self>, SemanticError> {
        let location = IndexLocation::for_vault(&vault_ctx.global.cache_dir, &vault_ctx.vault_path)?;
        if !location.state_path().exists() {
            return Ok(None);
        }
        Self::open_at(vault_ctx, location, options, false)
    }

    fn open_at(
        vault_ctx: &VaultContext,
        location: IndexLocation,
        options: IndexOptions<'_>,
        build: bool,
    ) -> Result<Option<Self>, SemanticError> {
        std::fs::create_dir_all(&location.dir)
            .map_err(|e| SemanticError::Io(format!("{}: {e}", location.dir.display())))?;
        let lock = IndexLock::acquire(&location.lock_path(), options.lock_timeout)?;

        let loaded = IndexState::load(&location.state_path()).unwrap_or_else(|e| {
            eprintln!("Warning: rebuilding the index, unreadable state: {e}");
            None
        });
        let mut index = Self {
            state: IndexState::new(location.vault_path.clone()),
            objects_dir: vault_ctx.objects_dir(),
            cerbo_dir: vault_ctx.cerbo_dir(),
            location,
            fingerprints: Fingerprints::new(),
            graph: None,
            live_state: false,
            _lock: lock,
        };

        if let Some(state) = loaded {
            if state.matches(&index.location) && index.location.graph_dir(state.generation).is_dir() {
                let generation = state.generation;
                index.state = state;
                index.live_state = true;
                // A store or table that cannot be read is rebuilt from the
                // files, like a missing one; it is never a lasting error.
                let graph = OxigraphIndex::open(&index.location.graph_dir(generation))
                    .inspect_err(|e| eprintln!("Warning: rebuilding the index, {e}"))
                    .ok();
                let table = fingerprints::load(&index.location.fingerprints_path(generation))
                    .unwrap_or_else(|e| {
                        eprintln!("Warning: rebuilding the index, {e}");
                        None
                    });
                match (graph, table) {
                    (Some(graph), Some(table)) => {
                        index.graph = Some(graph);
                        index.fingerprints = table;
                        index.state.stale |= index.location.stale_path().exists();
                        index.remove_other_generations();
                        return Ok(Some(index));
                    }
                    // Without its fingerprints the store cannot be caught up,
                    // but its similarity links still carry over into the
                    // rebuild.
                    (graph, _) => index.graph = graph,
                }
            } else {
                if build && state.vault_path != index.location.vault_path {
                    eprintln!(
                        "Warning: index {} belonged to {}; rebuilding it for {}",
                        index.location.dir.display(),
                        state.vault_path.display(),
                        index.location.vault_path.display()
                    );
                }
                // Keep counting generations, so the build never writes into
                // the directory the old state still names.
                index.state.generation = state.generation;
            }
        }
        if !build {
            return Ok(None);
        }

        index.rebuild(options.progress)?;
        Ok(Some(index))
    }

    /// The index's `state.json` as last written.
    pub fn state(&self) -> &IndexState {
        &self.state
    }

    /// Where this index lives.
    pub fn location(&self) -> &IndexLocation {
        &self.location
    }

    /// The live generation's store, for queries that must not see the vault's
    /// prefix overrides.
    pub(super) fn live_graph(&self) -> &OxigraphIndex {
        self.graph.as_ref().expect("an open index always has a live generation")
    }

    pub(super) fn graph(&mut self) -> &mut OxigraphIndex {
        self.graph.as_mut().expect("an open index always has a live generation")
    }

    /// Rebuild the whole index from the vault files into a new generation, then
    /// switch `state.json` to it atomically and delete the old one. Until the
    /// switch, the previous generation stays the live one (design D9).
    pub fn rebuild(&mut self, progress: Progress<'_>) -> Result<u64, SemanticError> {
        // Taken before the scan: an update that fails while this rebuild runs
        // sets the marker again, and the next operation rebuilds once more.
        let marked = take_stale_marker(&self.location);
        let result = self.build_generation(progress);
        if result.is_err() && (marked || self.state.stale) {
            set_stale_marker(&self.location);
        }
        result
    }

    fn build_generation(&mut self, progress: Progress<'_>) -> Result<u64, SemanticError> {
        let report = |p: BuildProgress| {
            if let Some(f) = progress {
                f(p)
            }
        };
        let generation = self.state.generation + 1;
        let graph_dir = self.location.graph_dir(generation);
        let fingerprints_path = self.location.fingerprints_path(generation);
        // Leftovers of an interrupted build of the same generation.
        remove_path(&graph_dir);
        remove_path(&fingerprints_path);

        // Stat before reading: a file changed in between is re-indexed by the
        // next catch-up rather than missed.
        let current = fingerprints::scan(&self.objects_dir)?;
        let mut uuids: Vec<String> = current.keys().cloned().collect();
        uuids.sort();
        report(BuildProgress::Started { objects: uuids.len() });
        let mut objects = read_objects(&self.objects_dir, &uuids, &|done, total| {
            report(BuildProgress::Read { done, total })
        });

        // Embeddings do not depend on the graph: carry the vectors, their
        // content hashes and the similarity links over from the live
        // generation, even when its store or fingerprints are lost. Without
        // one, the new generation starts without embeddings (design D6, D9).
        let vectors_path = self.location.vectors_path(generation);
        remove_path(&vectors_path);
        let mut content_hashes = Default::default();
        let mut similarity = None;
        let live_vectors = self.location.vectors_path(self.state.generation);
        if self.live_state && live_vectors.exists() {
            let bytes = std::fs::read(&live_vectors)
                .map_err(|e| SemanticError::Io(format!("{}: {e}", live_vectors.display())))?;
            crate::fsio::write_atomic(&vectors_path, &bytes)
                .map_err(|e| SemanticError::Io(format!("{}: {e}", vectors_path.display())))?;
            content_hashes = self.state.content_hashes.clone();
            similarity = match &self.graph {
                Some(live) => Some(live.similarity_triples()?),
                None => None,
            };
        }
        let carried = similarity.is_some();
        let gone = similarity.as_deref().map(|t| unlisted_objects(t, &current)).unwrap_or_default();
        if let Some(triples) = similarity {
            objects.push(ObjectGraph { uuid: SIMILARITY_GRAPH.into(), triples });
        }

        let mut graph = OxigraphIndex::open(&graph_dir)?;
        let mut triples = graph.load_all(&objects)?;
        // Links to and from pages trashed since the last refresh go now.
        if !gone.is_empty() {
            graph.apply(&[], &gone)?;
            triples = graph.triples()?;
        }
        // The live store was lost with its links: compute them again from the
        // carried vectors, which needs no model.
        if !carried && vectors_path.exists() {
            super::similarity::link_all(&mut graph, &vectors_path, &current)?;
            triples = graph.triples()?;
        }
        fingerprints::save(&fingerprints_path, &current)?;
        report(BuildProgress::Loaded { triples });

        let mut state = self.state.clone();
        state.format_version = super::FORMAT_VERSION;
        state.generation = generation;
        state.stale = false;
        state.vault_path = self.location.vault_path.clone();
        state.content_hashes = content_hashes;
        state.save(&self.location.state_path())?;

        self.graph = Some(graph);
        self.state = state;
        self.fingerprints = current;
        self.remove_other_generations();
        report(BuildProgress::Finished { generation, triples });
        Ok(triples)
    }

    /// Bring the index up to date with the vault files by a full fingerprint
    /// scan: re-index new or changed objects and drop vanished ones in one
    /// transaction. A stale index is rebuilt instead, since a failed update may
    /// have left any object half-indexed.
    pub fn catch_up(&mut self, progress: Progress<'_>) -> Result<CatchUp, SemanticError> {
        if self.state.stale {
            self.rebuild(progress)?;
            return Ok(CatchUp { rebuilt: true, ..CatchUp::default() });
        }
        let current = fingerprints::scan(&self.objects_dir)?;
        let (changed, removed) = fingerprints::diff(&self.fingerprints, &current);
        if changed.is_empty() && removed.is_empty() {
            return Ok(CatchUp::default());
        }
        let objects = read_objects(&self.objects_dir, &changed, &|_, _| {});
        self.graph().apply(&objects, &removed)?;
        self.fingerprints = current;
        self.save_fingerprints()?;
        Ok(CatchUp { upserted: changed.len(), removed: removed.len(), rebuilt: false })
    }

    /// Catch up with just the objects in `uuids`, which a file watcher saw
    /// change: each is re-indexed if its files changed, or dropped if it is
    /// gone. Falls back to [`Self::catch_up`] when the index is stale.
    pub fn catch_up_paths(&mut self, uuids: &[String]) -> Result<CatchUp, SemanticError> {
        if self.state.stale {
            return self.catch_up(None);
        }
        self.update_named(uuids, false)
    }

    /// Re-index the objects in `uuids` whatever their fingerprints say, or drop
    /// those that are gone: Cerbo itself just wrote their files (design D10),
    /// possibly within the file system's timestamp granularity.
    pub fn update_objects(&mut self, uuids: &[String]) -> Result<CatchUp, SemanticError> {
        self.update_named(uuids, true)
    }

    fn update_named(&mut self, uuids: &[String], force: bool) -> Result<CatchUp, SemanticError> {
        let mut changed = Vec::new();
        let mut removed = Vec::new();
        let requested: BTreeSet<&String> = uuids.iter().collect();
        for uuid in requested {
            if uuid::Uuid::parse_str(uuid).is_err() {
                continue;
            }
            let dir = self.objects_dir.join(uuid);
            if dir.is_dir() {
                let stamp = fingerprints::stamp_object(&dir)
                    .map_err(|e| SemanticError::Io(format!("{}: {e}", dir.display())))?;
                if force || self.fingerprints.get(uuid) != Some(&stamp) {
                    changed.push((uuid.clone(), stamp));
                }
            } else if force || self.fingerprints.contains_key(uuid) {
                removed.push(uuid.clone());
            }
        }
        if changed.is_empty() && removed.is_empty() {
            return Ok(CatchUp::default());
        }

        let changed_uuids: Vec<String> = changed.iter().map(|(u, _)| u.clone()).collect();
        let objects = read_objects(&self.objects_dir, &changed_uuids, &|_, _| {});
        self.graph().apply(&objects, &removed)?;
        for uuid in &removed {
            self.fingerprints.remove(uuid);
        }
        self.fingerprints.extend(changed);
        self.save_fingerprints()?;
        Ok(CatchUp { upserted: changed_uuids.len(), removed: removed.len(), rebuilt: false })
    }

    /// Number of triples in the live generation.
    pub fn triples(&self) -> Result<u64, SemanticError> {
        self.live_graph().triples()
    }

    /// Run a read-only SPARQL query with the vault's prefixes predeclared.
    pub fn query(&self, sparql: &str) -> Result<QueryOutput, SemanticError> {
        let map = PrefixMap::load(&self.cerbo_dir).unwrap_or_else(|e| {
            eprintln!("Warning: querying without the vault's prefixes: {e}");
            PrefixMap::default()
        });
        self.graph
            .as_ref()
            .expect("an open index always has a live generation")
            .query(sparql, &query_prefixes(&map))
    }

    fn save_fingerprints(&self) -> Result<(), SemanticError> {
        fingerprints::save(&self.location.fingerprints_path(self.state.generation), &self.fingerprints)
    }

    /// Delete every generation but the live one, and temporary files left by an
    /// interrupted write. Failures only cost disk space, so they are warnings.
    fn remove_other_generations(&self) {
        let Ok(entries) = std::fs::read_dir(&self.location.dir) else {
            return;
        };
        let live_graph = format!("graph-{}", self.state.generation);
        let live_fingerprints = format!("fingerprints-{}.bin", self.state.generation);
        let live_vectors = format!("vectors-{}.bin", self.state.generation);
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let generation_file =
                name.starts_with("graph-") || name.starts_with("fingerprints-") || name.starts_with("vectors-");
            let live = name == live_graph || name == live_fingerprints || name == live_vectors;
            let garbage = (generation_file && !live) || crate::fsio::is_temp_name(&name);
            if garbage {
                remove_path(&entry.path());
            }
        }
    }
}

/// The objects named in `triples` that are not in `current`.
fn unlisted_objects(triples: &[oxigraph::model::Triple], current: &Fingerprints) -> Vec<String> {
    let mut gone = BTreeSet::new();
    for t in triples {
        let subject = match &t.subject {
            oxigraph::model::NamedOrBlankNode::NamedNode(n) => Some(n.as_str()),
            _ => None,
        };
        let object = match &t.object {
            oxigraph::model::Term::NamedNode(n) => Some(n.as_str()),
            _ => None,
        };
        for iri in subject.into_iter().chain(object) {
            if let Some(uuid) = crate::rdf::uuid_from_object_iri(iri)
                && !current.contains_key(uuid)
            {
                gone.insert(uuid.to_string());
            }
        }
    }
    gone.into_iter().collect()
}

/// Remove the stale marker; true if it was there.
fn take_stale_marker(location: &IndexLocation) -> bool {
    match std::fs::remove_file(location.stale_path()) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            eprintln!("Warning: could not clear {}: {e}", location.stale_path().display());
            true
        }
    }
}

/// Create the stale marker, best effort.
fn set_stale_marker(location: &IndexLocation) {
    let path = location.stale_path();
    let result = std::fs::OpenOptions::new().write(true).create(true).truncate(false).open(&path);
    if let Err(e) = result {
        eprintln!("Warning: could not mark the index stale: {}: {e}", path.display());
    }
}

/// Remove a file or directory tree if it exists.
fn remove_path(path: &Path) {
    let result = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    if let Err(e) = result
        && e.kind() != std::io::ErrorKind::NotFound
    {
        eprintln!("Warning: could not remove {}: {e}", path.display());
    }
}

/// Answer one query the way the CLI does: open (building if needed), catch up
/// with the vault files, query, release.
pub fn query(vault_ctx: &VaultContext, sparql: &str, options: IndexOptions<'_>) -> Result<QueryOutput, SemanticError> {
    let mut index = SemanticIndex::open(vault_ctx, options)?;
    index.catch_up(options.progress)?;
    index.query(sparql)
}

/// Record in the index that Cerbo just wrote, created or trashed the objects in
/// `uuids` (design D10). This never fails: the files are already written and
/// the save stands. When the index cannot be updated — it is busy, say — a
/// warning is printed and `state.json` is marked stale, so the next operation
/// rebuilds the index. A vault without an index is left alone; the first
/// operation that needs one builds it from the files. In a process that
/// watches the vault (see [`set_sync_watched`]) a busy index is left to the
/// watcher instead.
pub fn sync_objects(vault_ctx: &VaultContext, uuids: &[String]) {
    if SYNC_WATCHED.load(Ordering::Relaxed) {
        sync_objects_waiting(vault_ctx, uuids, Duration::ZERO, true)
    } else {
        sync_objects_waiting(vault_ctx, uuids, LOCK_TIMEOUT, false)
    }
}

/// Whether [`sync_objects`] runs in a process whose file watcher reports every
/// write to an index worker (the desktop app).
static SYNC_WATCHED: AtomicBool = AtomicBool::new(false);

/// Declare that this process watches the vault it saves to and catches its
/// index up from the watcher's events (design D10, D12). A save then never
/// waits for the index, and one that finds it busy skips the update instead of
/// marking the index stale: the watcher reports the written object to the
/// worker anyway, and a stale marker would turn the worker's next update into
/// a full rebuild — again and again while the user keeps typing. Failures
/// other than a busy index still mark it stale.
pub fn set_sync_watched(watched: bool) {
    SYNC_WATCHED.store(watched, Ordering::Relaxed);
}

fn sync_objects_waiting(vault_ctx: &VaultContext, uuids: &[String], lock_timeout: Duration, watched: bool) {
    let options = IndexOptions { lock_timeout, ..IndexOptions::default() };
    let result = SemanticIndex::open_existing(vault_ctx, options).and_then(|index| match index {
        // A stale index is rebuilt by the next operation anyway.
        Some(mut index) if !index.state.stale => index.update_objects(uuids).map(drop),
        _ => Ok(()),
    });
    // The index, and with it the lock, is closed by now.
    match result {
        Ok(()) => {}
        // The holder is this process's worker or another Cerbo process; either
        // way the watcher has seen the write and the worker catches up.
        Err(SemanticError::IndexBusy) if watched => {}
        Err(e) => {
            eprintln!("Warning: {e}; the index is marked stale and is repaired on next use");
            mark_stale(vault_ctx);
        }
    }
}

/// Mark the vault's index stale, best effort and without the lock: the holder
/// may be stuck. Only the `STALE` marker is created; `state.json` is never
/// rewritten here, so a generation the holder switched to meanwhile, and its
/// embeddings, stay live. A holder that is rebuilding took the marker before
/// its scan, so the marker set now survives it (design D10, D12).
fn mark_stale(vault_ctx: &VaultContext) {
    match IndexLocation::for_vault(&vault_ctx.global.cache_dir, &vault_ctx.vault_path) {
        // A vault without an index has nothing to repair.
        Ok(location) if location.state_path().exists() => set_stale_marker(&location),
        Ok(_) => {}
        Err(e) => eprintln!("Warning: could not mark the index stale: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{object_create, object_delete, ObjectType};
    use crate::CerboContext;
    use oxigraph::model::{Literal, Term};
    use std::collections::BTreeMap;
    use std::fs;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::Mutex;
    use tempfile::TempDir;

    const MISSING: &str = "dead0000-0000-4000-8000-000000000137";

    /// A vault plus a private cache directory, so no test touches the real
    /// `$XDG_CACHE_HOME`.
    struct TestVault {
        tmp: TempDir,
        ctx: VaultContext,
    }

    impl TestVault {
        fn new() -> Self {
            let tmp = TempDir::new().unwrap();
            let vault_path = tmp.path().join("vault");
            fs::create_dir_all(vault_path.join(".cerbo").join("objects")).unwrap();
            let ctx = vault_at(&tmp, vault_path);
            Self { tmp, ctx }
        }

        /// The context object.rs writes through: config_dir is `.cerbo/`.
        fn cerbo(&self) -> CerboContext {
            CerboContext { config_dir: self.ctx.cerbo_dir(), cache_dir: self.ctx.global.cache_dir.clone() }
        }

        fn page(&self, title: &str, body: &str) -> String {
            let uuid = object_create(&self.cerbo(), None, ObjectType::Product, title.into()).unwrap();
            crate::page::page_write(&self.cerbo(), uuid.clone(), format!("# {title}\n\n{body}")).unwrap();
            uuid
        }

        /// Copy object `uuid` from `other` into this vault with plain file
        /// copies, the way file sync delivers it.
        fn copy_in(&self, other: &TestVault, uuid: &str) -> String {
            let dest = self.ctx.object_path(uuid);
            fs::create_dir_all(&dest).unwrap();
            for entry in fs::read_dir(other.ctx.object_path(uuid)).unwrap() {
                let entry = entry.unwrap();
                fs::copy(entry.path(), dest.join(entry.file_name())).unwrap();
            }
            uuid.to_string()
        }

        fn backlinks(&self, uuid: &str) -> Vec<(String, String)> {
            crate::semantic::backlinks(&self.ctx, uuid, IndexOptions::default())
                .unwrap()
                .into_iter()
                .map(|e| (e.uuid, e.title))
                .collect()
        }

        /// Backlinks straight from the index as it is, without a catch-up.
        fn indexed_backlinks(&self, uuid: &str) -> Vec<(String, String)> {
            SemanticIndex::open(&self.ctx, IndexOptions::default())
                .unwrap()
                .backlinks(uuid)
                .unwrap()
                .into_iter()
                .map(|e| (e.uuid, e.title))
                .collect()
        }

        fn page_md(&self, uuid: &str) -> PathBuf {
            self.ctx.object_path(uuid).join("page.md")
        }

        fn location(&self) -> IndexLocation {
            IndexLocation::for_vault(&self.ctx.global.cache_dir, &self.ctx.vault_path).unwrap()
        }

        fn query(&self, sparql: &str) -> QueryOutput {
            query(&self.ctx, sparql, IndexOptions::default()).unwrap()
        }

        fn titles(&self) -> Vec<String> {
            column(&self.query("SELECT ?t WHERE { ?p schema:name ?t }"))
        }
    }

    fn vault_at(tmp: &TempDir, vault_path: PathBuf) -> VaultContext {
        VaultContext {
            vault_path,
            global: CerboContext {
                config_dir: tmp.path().join("config"),
                cache_dir: tmp.path().join("cache"),
            },
        }
    }

    /// The single column of a SELECT, as sorted plain strings.
    fn column(output: &QueryOutput) -> Vec<String> {
        let QueryOutput::Solutions { rows, .. } = output else {
            panic!("expected solutions, got {output:?}");
        };
        let mut values: Vec<String> = rows
            .iter()
            .map(|row| match &row[0] {
                Some(Term::Literal(l)) => l.value().to_string(),
                Some(Term::NamedNode(n)) => n.as_str().to_string(),
                other => format!("{other:?}"),
            })
            .collect();
        values.sort();
        values
    }

    fn ask(vault: &TestVault, sparql: &str) -> bool {
        match vault.query(sparql) {
            QueryOutput::Boolean(b) => b,
            other => panic!("expected a boolean, got {other:?}"),
        }
    }

    fn cites(vault: &TestVault, source: &str, target: &str) -> bool {
        ask(vault, &format!("ASK {{ <cerbo://objects/{source}> schema:citation <cerbo://objects/{target}> }}"))
    }

    /// Every file under the vault with its size and mtime.
    fn snapshot(dir: &Path) -> BTreeMap<PathBuf, (u64, std::time::SystemTime)> {
        walkdir::WalkDir::new(dir)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let meta = e.metadata().unwrap();
                (e.path().to_path_buf(), (meta.len(), meta.modified().unwrap()))
            })
            .collect()
    }

    // ── 2.4 catch-up ───────────────────────────────────────────

    #[test]
    fn query_sees_files_added_edited_and_deleted_behind_cerbos_back() {
        let vault = TestVault::new();
        let target = vault.page("Berlin", "");
        assert_eq!(vault.titles(), ["Berlin"]);

        // Page arrives through file sync: a copied directory, no Cerbo call.
        let synced = TestVault::new();
        let arriving = synced.page("Synced", &format!("[B](cerbo://{target})"));
        let dest = vault.ctx.object_path(&arriving);
        fs::create_dir_all(&dest).unwrap();
        for file in ["meta.ttl", "page.md"] {
            fs::copy(synced.ctx.object_path(&arriving).join(file), dest.join(file)).unwrap();
        }
        assert_eq!(vault.titles(), ["Berlin", "Synced"]);
        assert!(cites(&vault, &arriving, &target), "synced page's link must be indexed");

        // Link removed in a text editor.
        fs::write(vault.page_md(&arriving), "# Synced\n\nno more links here\n").unwrap();
        assert!(!cites(&vault, &arriving, &target), "removed link still visible");

        // Directory deleted by hand.
        fs::remove_dir_all(&dest).unwrap();
        assert_eq!(vault.titles(), ["Berlin"]);
        assert!(!ask(&vault, &format!("ASK {{ GRAPH <cerbo://graphs/{arriving}> {{ ?s ?p ?o }} }}")));
    }

    #[test]
    fn trashed_object_leaves_the_index_with_its_links() {
        let vault = TestVault::new();
        let target = vault.page("Target", "");
        let source = vault.page("Source", &format!("[T](cerbo://{target})"));
        assert!(cites(&vault, &source, &target));

        object_delete(&vault.cerbo(), &source).unwrap();

        assert_eq!(vault.titles(), ["Target"]);
        assert!(!cites(&vault, &source, &target));
    }

    #[test]
    fn incremental_catch_up_touches_only_the_named_objects() {
        let vault = TestVault::new();
        let a = vault.page("A", "");
        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();

        // Written behind Cerbo's back, as a watcher would report them: Cerbo's
        // own save hooks would wait for the lock this test holds.
        let elsewhere = TestVault::new();
        let b = vault.copy_in(&elsewhere, &elsewhere.page("B", &format!("[A](cerbo://{a})")));
        vault.copy_in(&elsewhere, &elsewhere.page("C", ""));
        let report = index.catch_up_paths(std::slice::from_ref(&b)).unwrap();
        assert_eq!(report, CatchUp { upserted: 1, removed: 0, rebuilt: false });
        assert_eq!(column(&index.query("SELECT ?t WHERE { ?p schema:name ?t }").unwrap()), ["A", "B"]);

        // An unchanged object is a no-op; a vanished one is removed.
        fs::remove_dir_all(vault.ctx.object_path(&b)).unwrap();
        let report = index.catch_up_paths(&[a.clone(), b.clone(), "not-a-uuid".into()]).unwrap();
        assert_eq!(report, CatchUp { upserted: 0, removed: 1, rebuilt: false });
        assert_eq!(column(&index.query("SELECT ?t WHERE { ?p schema:name ?t }").unwrap()), ["A"]);

        // The fingerprints were kept current: a full scan finds only C.
        assert_eq!(index.catch_up(None).unwrap(), CatchUp { upserted: 1, removed: 0, rebuilt: false });
        assert_eq!(index.catch_up(None).unwrap(), CatchUp::default());
    }

    #[test]
    fn stale_index_is_rebuilt_on_next_use() {
        let vault = TestVault::new();
        vault.page("A", "");
        assert_eq!(vault.titles(), ["A"]);

        let location = vault.location();
        let mut state = IndexState::load(&location.state_path()).unwrap().unwrap();
        state.stale = true;
        state.save(&location.state_path()).unwrap();
        vault.page("B", "");

        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        assert!(index.catch_up(None).unwrap().rebuilt);
        assert!(!index.state().stale);
        assert_eq!(index.state().generation, state.generation + 1);
        assert_eq!(column(&index.query("SELECT ?t WHERE { ?p schema:name ?t }").unwrap()), ["A", "B"]);
    }

    #[test]
    fn index_operations_never_write_into_the_vault() {
        let vault = TestVault::new();
        let a = vault.page("A", "");
        vault.page("B", &format!("[A](cerbo://{a}) [gone](cerbo://{MISSING})"));
        let before = snapshot(&vault.ctx.vault_path);

        vault.titles();
        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        index.rebuild(None).unwrap();
        drop(index);

        assert_eq!(snapshot(&vault.ctx.vault_path), before);
        assert!(!vault.location().dir.starts_with(&vault.ctx.vault_path));
    }

    // ── 2.5 automatic build ────────────────────────────────────

    #[test]
    fn query_before_first_index_builds_it_with_progress() {
        let vault = TestVault::new();
        vault.page("Index", "");
        assert!(!vault.location().dir.exists());

        let seen = Mutex::new(Vec::new());
        let progress = |p: BuildProgress| seen.lock().unwrap().push(p);
        let options = IndexOptions { progress: Some(&progress), ..IndexOptions::default() };
        let titles = column(&query(&vault.ctx, "SELECT ?t WHERE { ?p schema:name ?t }", options).unwrap());

        assert_eq!(titles, ["Index"]);
        let seen = seen.into_inner().unwrap();
        assert_eq!(seen.first(), Some(&BuildProgress::Started { objects: 1 }));
        assert!(seen.contains(&BuildProgress::Read { done: 1, total: 1 }), "{seen:?}");
        assert!(matches!(seen.last(), Some(BuildProgress::Finished { generation: 1, .. })), "{seen:?}");
        let state = IndexState::load(&vault.location().state_path()).unwrap().unwrap();
        assert_eq!(state.generation, 1);
        assert_eq!(state.format_version, super::super::FORMAT_VERSION);
    }

    #[test]
    fn index_from_an_older_version_is_rebuilt() {
        let vault = TestVault::new();
        vault.page("A", "");
        vault.titles();
        let location = vault.location();
        let mut state = IndexState::load(&location.state_path()).unwrap().unwrap();
        state.format_version = 0;
        state.save(&location.state_path()).unwrap();

        let index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();

        assert_eq!(index.state().format_version, super::super::FORMAT_VERSION);
        assert_eq!(index.state().generation, 2);
        assert!(!location.graph_dir(1).exists(), "old-format generation left behind");
        assert_eq!(column(&index.query("SELECT ?t WHERE { ?p schema:name ?t }").unwrap()), ["A"]);
    }

    #[test]
    fn moved_vault_gets_a_new_index() {
        let vault = TestVault::new();
        vault.page("A", "");
        vault.titles();
        let old_location = vault.location();

        let moved_path = vault.tmp.path().join("moved");
        fs::rename(&vault.ctx.vault_path, &moved_path).unwrap();
        let moved = vault_at(&vault.tmp, moved_path);
        let built = Mutex::new(false);
        let progress = |p: BuildProgress| {
            if matches!(p, BuildProgress::Finished { .. }) {
                *built.lock().unwrap() = true;
            }
        };
        let options = IndexOptions { progress: Some(&progress), ..IndexOptions::default() };
        let titles = column(&query(&moved, "SELECT ?t WHERE { ?p schema:name ?t }", options).unwrap());

        assert_eq!(titles, ["A"]);
        assert!(*built.lock().unwrap(), "no index was built for the new path");
        let new_location = IndexLocation::for_vault(&moved.global.cache_dir, &moved.vault_path).unwrap();
        assert_ne!(new_location.dir, old_location.dir);
        assert!(new_location.state_path().exists());
    }

    #[test]
    fn key_collision_rebuilds_for_the_vault_being_opened() {
        let vault = TestVault::new();
        vault.page("Mine", "");
        vault.titles();
        let location = vault.location();
        let mut state = IndexState::load(&location.state_path()).unwrap().unwrap();
        state.vault_path = PathBuf::from("/elsewhere/vault-137");
        state.save(&location.state_path()).unwrap();

        let index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();

        assert_eq!(index.state().vault_path, location.vault_path);
        assert_eq!(index.state().generation, 2);
    }

    // ── 2.6 atomic rebuild ─────────────────────────────────────

    #[test]
    fn interrupted_rebuild_keeps_the_previous_generation() {
        let vault = TestVault::new();
        vault.page("Old", "");
        vault.titles();
        // Not through Cerbo, whose save hook would index it into generation 1.
        let elsewhere = TestVault::new();
        vault.copy_in(&elsewhere, &elsewhere.page("New", ""));

        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        let abort = |p: BuildProgress| {
            if matches!(p, BuildProgress::Loaded { .. }) {
                panic!("simulated crash before the switch");
            }
        };
        let crashed = catch_unwind(AssertUnwindSafe(|| index.rebuild(Some(&abort))));
        assert!(crashed.is_err());
        drop(index);

        let location = vault.location();
        assert!(location.graph_dir(2).exists(), "the half-switched generation is on disk");
        let index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        assert_eq!(index.state().generation, 1);
        assert_eq!(
            column(&index.query("SELECT ?t WHERE { ?p schema:name ?t }").unwrap()),
            ["Old"],
            "must answer from the previous complete generation"
        );
        assert!(!location.graph_dir(2).exists(), "garbage generation not cleaned up");
    }

    #[test]
    fn completed_rebuild_switches_generation_and_deletes_the_old_one() {
        let vault = TestVault::new();
        vault.page("A", "");
        vault.titles();
        let location = vault.location();

        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        index.rebuild(None).unwrap();

        assert_eq!(index.state().generation, 2);
        assert!(location.graph_dir(2).is_dir());
        assert!(location.fingerprints_path(2).is_file());
        assert!(!location.graph_dir(1).exists());
        assert!(!location.fingerprints_path(1).exists());
    }

    #[test]
    fn deleting_the_index_loses_nothing() {
        let vault = TestVault::new();
        let a = vault.page("A", "[[x]]");
        vault.page("B", &format!("[A](cerbo://{a}) [Berlin]{{schema:Place}}"));
        let everything = "SELECT ?g ?s ?p ?o WHERE { GRAPH ?g { ?s ?p ?o } FILTER(!isBlank(?s) && !isBlank(?o)) } ORDER BY ?g ?s ?p ?o";
        let before = vault.query(everything);

        fs::remove_dir_all(vault.location().dir).unwrap();

        assert_eq!(vault.query(everything), before);
    }

    // ── 2.7 queries (spec graph-query) ─────────────────────────

    #[test]
    fn list_page_titles() {
        let vault = TestVault::new();
        vault.page("Index", "");
        vault.page("Berlin", "");
        assert_eq!(vault.titles(), ["Berlin", "Index"]);
    }

    #[test]
    fn two_vaults_have_separate_indexes() {
        let one = TestVault::new();
        one.page("One", "");
        let two = TestVault::new();
        two.page("Two", "");

        assert_eq!(one.titles(), ["One"]);
        assert_eq!(two.titles(), ["Two"]);
        assert_ne!(one.location().dir, two.location().dir);
    }

    #[test]
    fn cerbo_schema_and_rdf_prefixes_are_predeclared() {
        let vault = TestVault::new();
        vault.page("Trip", "[Berlin]{schema:Place}");

        assert!(ask(&vault, "ASK { FILTER(STR(cerbo:slug) = \"cerbo://ontology/slug\") }"));
        assert!(ask(&vault, "ASK { ?p cerbo:slug ?s }"));
        assert!(ask(&vault, "ASK { ?a a schema:Place }"));
        assert!(ask(&vault, "ASK { FILTER(schema:Place = <https://schema.org/Place>) }"));
        assert!(ask(&vault, "ASK { FILTER(rdfs:label = <http://www.w3.org/2000/01/rdf-schema#label>) }"));
        assert!(ask(&vault, "ASK { ?p rdf:type ?c }"));
        assert!(ask(&vault, "ASK { FILTER(xsd:string = <http://www.w3.org/2001/XMLSchema#string>) }"));
    }

    #[test]
    fn vault_map_prefix_overrides_builtin_and_query_prefix_overrides_both() {
        let vault = TestVault::new();
        vault.page("A", "");
        let mut map = PrefixMap::default();
        map.register("ex", "https://example.org/vocab#", None);
        map.register("foaf", "https://example.org/my-foaf#", None);
        map.save(&vault.ctx.cerbo_dir()).unwrap();

        assert!(ask(&vault, "ASK { FILTER(ex:Person = <https://example.org/vocab#Person>) }"));
        assert!(ask(&vault, "ASK { FILTER(foaf:Person = <https://example.org/my-foaf#Person>) }"));
        assert!(ask(
            &vault,
            "PREFIX ex: <https://other.example/#>\nASK { FILTER(ex:Person = <https://other.example/#Person>) }"
        ));
    }

    #[test]
    fn ask_and_construct() {
        let vault = TestVault::new();
        let uuid = vault.page("Index", "");

        assert!(ask(&vault, "ASK { ?s schema:name \"Index\" }"));
        assert!(!ask(&vault, "ASK { ?s schema:name \"Nope\" }"));

        let QueryOutput::Graph(triples) =
            vault.query("CONSTRUCT { ?s schema:name ?t } WHERE { ?s schema:name ?t }")
        else {
            panic!("expected a graph");
        };
        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].subject, crate::rdf::object_iri(&uuid).into());
        assert_eq!(triples[0].object, Term::from(Literal::new_simple_literal("Index")));
        // The result is plain RDF that Cerbo's Turtle writer can emit.
        assert_eq!(crate::rdf::parse(&crate::rdf::serialize(&triples)).unwrap(), triples);
    }

    #[test]
    fn union_default_graph_only_without_from() {
        let vault = TestVault::new();
        let a = vault.page("A", "");
        vault.page("B", "");
        assert_eq!(vault.titles(), ["A", "B"]);

        let from_a = format!("SELECT ?t FROM <cerbo://graphs/{a}> WHERE {{ ?p schema:name ?t }}");
        assert_eq!(column(&vault.query(&from_a)), ["A"]);
        let from_named = "SELECT ?t FROM NAMED <cerbo://graphs/none> WHERE { ?p schema:name ?t }";
        assert_eq!(column(&vault.query(from_named)), Vec::<String>::new());
    }

    #[test]
    fn updates_are_rejected_and_change_nothing() {
        let vault = TestVault::new();
        vault.page("A", "");
        let count = "SELECT (COUNT(*) AS ?n) WHERE { GRAPH ?g { ?s ?p ?o } }";
        let before = vault.query(count);

        for update in [
            "INSERT DATA { <a:b> <a:c> \"d\" }",
            "DELETE WHERE { ?s ?p ?o }",
            "CLEAR ALL",
            "DELETE { ?s schema:name ?t } WHERE { ?s schema:name ?t }",
        ] {
            let err = query(&vault.ctx, update, IndexOptions::default()).unwrap_err();
            assert_eq!(err, SemanticError::ReadOnlyQuery, "{update}");
            assert!(err.to_string().contains("changed through files"), "{err}");
        }
        assert_eq!(vault.query(count), before);
    }

    #[test]
    fn syntax_and_evaluation_errors_are_reported() {
        let vault = TestVault::new();
        vault.page("A", "");

        let err = query(&vault.ctx, "SELEC ?s WHERE { ?s ?p ?o }", IndexOptions::default()).unwrap_err();
        let SemanticError::QuerySyntax(message) = &err else {
            panic!("expected a syntax error, got {err:?}");
        };
        assert!(message.contains("1:1"), "position missing: {message}");

        // SERVICE has no handler: Cerbo never makes network calls for a query.
        let err = query(
            &vault.ctx,
            "SELECT * WHERE { SERVICE <http://example.org/sparql> { ?s ?p ?o } }",
            IndexOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(err, SemanticError::QueryEvaluation(_)), "{err:?}");
    }

    #[test]
    fn broken_links_and_late_targets() {
        let vault = TestVault::new();
        let source = vault.page("Source", &format!("[later](cerbo://{MISSING})"));

        let broken = "SELECT ?t WHERE { ?a schema:citation ?t FILTER NOT EXISTS { ?t a ?c } }";
        assert_eq!(column(&vault.query(broken)), [format!("cerbo://objects/{MISSING}")]);

        // The target is created later with that UUID.
        object_create(&vault.cerbo(), Some(MISSING.into()), ObjectType::Product, "Later".into()).unwrap();
        assert!(cites(&vault, &source, MISSING));
        assert_eq!(column(&vault.query(broken)), Vec::<String>::new());
    }

    // ── 3.1–3.3 backlinks from the index ───────────────────────

    #[test]
    fn page_linked_from_two_pages() {
        let vault = TestVault::new();
        let target = vault.page("Target", "");
        let a = vault.page("Beta", &format!("[T](cerbo://{target}) and again [T](cerbo://objects/{target})"));
        let b = vault.page("Alpha", &format!("[T](cerbo://{})", target.to_uppercase()));
        vault.page("Unrelated", "no links");

        assert_eq!(vault.backlinks(&target), [(b, "Alpha".to_string()), (a, "Beta".to_string())]);
    }

    #[test]
    fn page_linked_from_nowhere() {
        let vault = TestVault::new();
        let target = vault.page("Target", "");
        vault.page("Other", "");

        assert!(vault.backlinks(&target).is_empty());
    }

    #[test]
    fn linking_page_moved_to_trash() {
        let vault = TestVault::new();
        let target = vault.page("Target", "");
        let a = vault.page("A", &format!("[T](cerbo://{target})"));
        let b = vault.page("B", &format!("[T](cerbo://{target})"));
        assert_eq!(vault.backlinks(&target).len(), 2);

        object_delete(&vault.cerbo(), &a).unwrap();

        // The delete hook already dropped A's graph; no catch-up is needed.
        assert_eq!(vault.indexed_backlinks(&target), [(b.clone(), "B".to_string())]);
        assert_eq!(vault.backlinks(&target), [(b, "B".to_string())]);
    }

    #[test]
    fn unknown_object_has_no_backlinks_but_an_error() {
        let vault = TestVault::new();
        vault.page("A", "");

        for id in ["00000000-0000-0000-0000-000000000000", "not-a-uuid"] {
            let err = crate::semantic::backlinks(&vault.ctx, id, IndexOptions::default()).unwrap_err();
            assert_eq!(err, SemanticError::ObjectNotFound(id.to_string()));
            assert!(err.to_string().contains("Object not found"), "{err}");
        }
    }

    #[test]
    fn saving_a_link_shows_the_backlink_without_reindexing_and_leaves_the_target_untouched() {
        let vault = TestVault::new();
        let target = vault.page("Target", "");
        let source = vault.page("Source", "");
        // Saves into a vault without an index leave the cache alone.
        assert!(!vault.location().dir.exists());
        assert!(vault.backlinks(&target).is_empty());
        let target_files = snapshot(&vault.ctx.object_path(&target));

        crate::page::page_write(&vault.cerbo(), source.clone(), format!("# Source\n\n[T](cerbo://{target})")).unwrap();
        assert_eq!(vault.indexed_backlinks(&target), [(source.clone(), "Source".to_string())]);

        crate::page::page_write(&vault.cerbo(), source.clone(), "# Source\n\nno link".into()).unwrap();
        assert!(vault.indexed_backlinks(&target).is_empty());

        // The hook kept the fingerprints current too: nothing left to catch up.
        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        assert_eq!(index.catch_up(None).unwrap(), CatchUp::default());
        drop(index);
        assert_eq!(snapshot(&vault.ctx.object_path(&target)), target_files, "a target file changed");
    }

    #[test]
    fn index_failure_marks_it_stale_without_failing_the_write() {
        let vault = TestVault::new();
        let target = vault.page("Target", "");
        let source = vault.page("Source", "");
        assert!(vault.backlinks(&target).is_empty());
        let location = vault.location();

        // Another process holds the index for longer than the save waits.
        let lock = IndexLock::acquire(&location.lock_path(), LOCK_TIMEOUT).unwrap();
        let content = format!("# Source\n\n[T](cerbo://{target})");
        crate::page::page_write(&vault.cerbo(), source.clone(), content.clone())
            .expect("an index failure must not fail the save");
        assert_eq!(fs::read_to_string(vault.page_md(&source)).unwrap(), content);
        assert!(location.stale_path().exists());
        drop(lock);

        // The next query repairs it.
        assert_eq!(vault.backlinks(&target), [(source, "Source".to_string())]);
        assert!(!location.stale_path().exists());
        assert!(!IndexState::load(&location.state_path()).unwrap().unwrap().stale);
    }

    #[test]
    fn a_save_that_may_not_wait_marks_stale_without_rolling_state_back() {
        let vault = TestVault::new();
        let a = vault.page("A", "");
        vault.titles();
        let location = vault.location();

        // The holder switches to generation 2 and keeps the index open.
        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        index.rebuild(None).unwrap();
        let live = IndexState::load(&location.state_path()).unwrap().unwrap();
        assert_eq!(live.generation, 2);

        let started = std::time::Instant::now();
        sync_objects_waiting(&vault.ctx, std::slice::from_ref(&a), Duration::ZERO, false);
        assert!(started.elapsed() < Duration::from_secs(1), "a save must not wait for the index");
        assert_eq!(IndexState::load(&location.state_path()).unwrap().unwrap(), live, "state.json was rewritten");
        assert!(location.stale_path().exists());
        drop(index);

        let index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        assert!(index.state().stale);
        assert_eq!(index.state().generation, 2);
        assert!(location.graph_dir(2).is_dir());
        drop(index);

        assert_eq!(vault.titles(), ["A"]);
        assert!(!location.stale_path().exists());
        assert_eq!(IndexState::load(&location.state_path()).unwrap().unwrap().generation, 3);
    }

    #[test]
    fn a_watched_save_that_finds_the_index_busy_leaves_it_to_the_watcher() {
        let vault = TestVault::new();
        let a = vault.page("A", "");
        vault.titles();
        let location = vault.location();

        let index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        let started = std::time::Instant::now();
        sync_objects_waiting(&vault.ctx, std::slice::from_ref(&a), Duration::ZERO, true);
        assert!(started.elapsed() < Duration::from_secs(1), "a save must not wait for the index");
        assert!(!location.stale_path().exists(), "a busy index must not be marked stale");
        drop(index);

        // The worker's catch-up from the watcher's event picks the save up
        // without a rebuild.
        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        assert!(!index.state().stale);
        assert!(!index.catch_up_paths(std::slice::from_ref(&a)).unwrap().rebuilt);
        assert_eq!(index.state().generation, 1);
    }

    #[test]
    fn marker_set_during_a_rebuild_survives_it() {
        let vault = TestVault::new();
        let a = vault.page("A", "");
        vault.titles();
        let location = vault.location();

        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        let ctx = vault.ctx.clone();
        let save_meanwhile = |p: BuildProgress| {
            if matches!(p, BuildProgress::Loaded { .. }) {
                sync_objects_waiting(&ctx, std::slice::from_ref(&a), Duration::ZERO, false);
            }
        };
        index.rebuild(Some(&save_meanwhile)).unwrap();
        drop(index);

        assert!(location.stale_path().exists(), "the save's marker was lost");
        assert!(SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap().state().stale);
    }

    #[test]
    fn unreadable_store_is_rebuilt() {
        let vault = TestVault::new();
        vault.page("A", "");
        vault.titles();
        let location = vault.location();
        let graph = location.graph_dir(1);
        fs::remove_dir_all(&graph).unwrap();
        fs::create_dir_all(&graph).unwrap();
        fs::write(graph.join("CURRENT"), "MANIFEST-000137\n").unwrap();
        assert!(OxigraphIndex::open(&graph).is_err(), "the store must be unreadable for this test");

        assert_eq!(vault.titles(), ["A"]);
        assert_eq!(IndexState::load(&location.state_path()).unwrap().unwrap().generation, 2);
        assert!(!graph.exists());
    }

    #[test]
    fn lost_fingerprints_are_rebuilt() {
        let vault = TestVault::new();
        vault.page("A", "");
        vault.titles();
        let location = vault.location();
        fs::write(location.fingerprints_path(1), "not a table").unwrap();

        assert_eq!(vault.titles(), ["A"]);
        assert_eq!(IndexState::load(&location.state_path()).unwrap().unwrap().generation, 2);
    }
}
