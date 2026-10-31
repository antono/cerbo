//! Derived semantic index of a vault: the RDF graph index and local text
//! embeddings. Everything here is rebuilt from vault files and lives in the
//! user cache, never inside the vault.
//!
//! Layout of one vault's index directory (design D3, D9, D12, D13):
//!
//! ```text
//! $XDG_CACHE_HOME/cerbo/index/<sha256(canonical vault path)[..16]>/
//!   LOCK                      advisory lock, held for one operation
//!   state.json                IndexState: generation, format, stale flag, …
//!   STALE                     set without the lock when an update failed
//!   graph-<gen>/              Oxigraph (RocksDB) store, one named graph per object
//!   fingerprints-<gen>.bin    per-object file stats for the catch-up step
//!   vectors-<gen>.bin         page embeddings, once `cerbo index --embeddings` ran
//! ```

mod backlinks;
mod embed;
mod fingerprints;
mod graph;
mod lock;
pub mod model;
mod query;
mod session;
mod similarity;
mod vectors;

pub use backlinks::{backlinks, BacklinkEntry};
#[cfg(feature = "embeddings")]
pub use embed::FastEmbedder;
#[cfg(any(test, feature = "test-utils"))]
pub use embed::MockEmbedder;
#[cfg(feature = "embeddings")]
pub use embed::default_embedder;
pub use embed::{content_hash, passage_text, query_text, EmbedProgress, Embedder, LazyEmbedder, MOCK_EMBEDDER_ENV};
pub use similarity::{
    refresh_embeddings, search, similar, EmbeddingRefresh, SearchHit, SearchOptions, SIMILARITY_MIN_SCORE,
    SIMILARITY_NEIGHBOURS,
};
pub use vectors::{FlatVectors, VectorIndex};
pub use graph::{link_targets, ObjectGraph, OxigraphIndex};
pub use lock::IndexLock;
pub use query::{query_prefixes, QueryOutput};
pub use session::{query, set_sync_watched, sync_objects, BuildProgress, CatchUp, IndexOptions, Progress, SemanticIndex};

use oxigraph::model::Triple;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// On-disk format of the index. An index written with another version is
/// rebuilt automatically, never read.
pub const FORMAT_VERSION: u32 = 1;

/// How long an operation waits for another process to release the index.
pub const LOCK_TIMEOUT: Duration = Duration::from_secs(5);

/// Hex characters of the vault path's SHA-256 used as the directory name.
const VAULT_KEY_LEN: usize = 16;

/// Why an index operation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticError {
    /// Another process held the index for longer than [`LOCK_TIMEOUT`].
    IndexBusy,
    /// The query is a SPARQL UPDATE; vault data changes only through files.
    ReadOnlyQuery,
    /// The query does not parse; the message includes the position.
    QuerySyntax(String),
    /// The query parsed but failed while running.
    QueryEvaluation(String),
    /// No live object has this id.
    ObjectNotFound(String),
    /// The embedding model cannot be downloaded, verified or loaded.
    ModelUnavailable(String),
    /// The vault has no embeddings yet: `cerbo index --embeddings` creates them.
    NoEmbeddings,
    /// A `--type`/`--annotated` filter is not a known `prefix:Name` or IRI.
    InvalidFilter(String),
    /// Reading the vault or reading/writing the index failed.
    Io(String),
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexBusy => write!(
                f,
                "index busy: another Cerbo process is using it, try again shortly"
            ),
            Self::ReadOnlyQuery => write!(
                f,
                "SPARQL updates are not allowed: vault data is changed through files, not the index"
            ),
            Self::QuerySyntax(e) => write!(f, "SPARQL syntax error: {e}"),
            Self::QueryEvaluation(e) => write!(f, "SPARQL evaluation error: {e}"),
            Self::ObjectNotFound(uuid) => write!(f, "Object not found: {uuid}"),
            Self::ModelUnavailable(e) => write!(f, "embedding model unavailable: {e}"),
            Self::NoEmbeddings => write!(
                f,
                "this vault has no embeddings yet: run `cerbo index --embeddings` first"
            ),
            Self::InvalidFilter(e) => write!(f, "invalid filter: {e}"),
            Self::Io(e) => write!(f, "semantic index: {e}"),
        }
    }
}

impl std::error::Error for SemanticError {}

impl From<SemanticError> for String {
    fn from(e: SemanticError) -> Self {
        e.to_string()
    }
}

/// A store for object graphs (design D2). Kept engine-neutral so another
/// engine can replace Oxigraph without touching callers.
pub trait GraphIndex {
    /// Replace the graphs of `upserts` and drop the graphs of `removals`, all in
    /// one transaction.
    fn apply(&mut self, upserts: &[ObjectGraph], removals: &[String]) -> Result<(), SemanticError>;

    /// Replace one object's graph with `graph`.
    fn upsert_object(&mut self, graph: &ObjectGraph) -> Result<(), SemanticError> {
        self.apply(std::slice::from_ref(graph), &[])
    }

    /// Drop one object's graph, including its outgoing links.
    fn remove_object(&mut self, uuid: &str) -> Result<(), SemanticError> {
        self.apply(&[], &[uuid.to_string()])
    }

    /// Fill an empty index with `objects` in one bulk load; returns the number
    /// of triples loaded.
    fn load_all(&mut self, objects: &[ObjectGraph]) -> Result<u64, SemanticError>;

    /// Run a read-only SPARQL query with `prefixes` predeclared.
    fn query(&self, sparql: &str, prefixes: &[(String, String)]) -> Result<QueryOutput, SemanticError>;

    /// Every triple of the similarity graph `cerbo://graphs/similarity`.
    fn similarity_triples(&self) -> Result<Vec<Triple>, SemanticError>;

    /// The `(source, target)` UUIDs of every `cerbo:similarTo` link.
    fn similarity_edges(&self) -> Result<Vec<(String, String)>, SemanticError>;

    /// Drop the similarity links of `sources` (with their score nodes) and add
    /// `edges` as `(source, target, score)`, in one transaction (design D7).
    fn replace_similarity(&mut self, sources: &[String], edges: &[(String, String, f32)]) -> Result<(), SemanticError>;
}

/// Where one vault's index lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexLocation {
    /// `<cache>/index/<vault-key>/`.
    pub dir: PathBuf,
    /// The vault's canonical path, which the key is derived from.
    pub vault_path: PathBuf,
}

impl IndexLocation {
    /// The index directory of the vault at `vault_path`, under `cache_dir`
    /// (`$XDG_CACHE_HOME/cerbo`). The directory is not created here.
    pub fn for_vault(cache_dir: &Path, vault_path: &Path) -> Result<Self, SemanticError> {
        let vault_path = vault_path.canonicalize().map_err(|e| {
            SemanticError::Io(format!("cannot resolve vault path {}: {e}", vault_path.display()))
        })?;
        let dir = cache_dir.join("index").join(vault_key(&vault_path));
        Ok(Self { dir, vault_path })
    }

    pub fn lock_path(&self) -> PathBuf {
        self.dir.join("LOCK")
    }

    pub fn state_path(&self) -> PathBuf {
        self.dir.join("state.json")
    }

    /// Marker that an update failed and the index must be rebuilt. It is a
    /// file of its own, not a field rewritten in `state.json`, because it is
    /// set without the lock: rewriting `state.json` there could undo a switch
    /// to a new generation that the lock holder made in the meantime.
    pub fn stale_path(&self) -> PathBuf {
        self.dir.join("STALE")
    }

    pub fn graph_dir(&self, generation: u64) -> PathBuf {
        self.dir.join(format!("graph-{generation}"))
    }

    pub fn fingerprints_path(&self, generation: u64) -> PathBuf {
        self.dir.join(format!("fingerprints-{generation}.bin"))
    }

    pub fn vectors_path(&self, generation: u64) -> PathBuf {
        self.dir.join(format!("vectors-{generation}.bin"))
    }
}

/// First [`VAULT_KEY_LEN`] hex characters of the SHA-256 of a canonical vault
/// path. Two paths can collide; `state.json` records the full path to notice.
pub fn vault_key(canonical_vault_path: &Path) -> String {
    let digest = Sha256::digest(canonical_vault_path.as_os_str().as_encoded_bytes());
    let mut key: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    key.truncate(VAULT_KEY_LEN);
    key
}

/// `state.json`: which generation is live and whether it can be trusted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexState {
    /// [`FORMAT_VERSION`] of the build that wrote the index.
    pub format_version: u32,
    /// The live generation: `graph-<generation>/` and its fingerprints.
    pub generation: u64,
    /// Set when an update failed; the next operation re-indexes everything.
    /// Also set on load when [`IndexLocation::stale_path`] exists.
    #[serde(default)]
    pub stale: bool,
    /// Canonical path of the vault this index was built from.
    pub vault_path: PathBuf,
    /// Page UUID → hash of the content last embedded (design D6).
    #[serde(default)]
    pub content_hashes: BTreeMap<String, String>,
}

impl IndexState {
    /// The state of an index that has no generation yet.
    pub fn new(vault_path: PathBuf) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            generation: 0,
            stale: false,
            vault_path,
            content_hashes: BTreeMap::new(),
        }
    }

    /// Read `state.json`. A missing file is `None`.
    pub fn load(path: &Path) -> Result<Option<Self>, SemanticError> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(SemanticError::Io(format!("{}: {e}", path.display()))),
        };
        serde_json::from_str(&content)
            .map(Some)
            .map_err(|e| SemanticError::Io(format!("{}: {e}", path.display())))
    }

    /// Replace `state.json` with the durable atomic write, so a reader sees the
    /// old or the new generation, never a mix (design D9).
    pub fn save(&self, path: &Path) -> Result<(), SemanticError> {
        let json = serde_json::to_string_pretty(self).expect("index state serialises");
        crate::fsio::write_atomic_str(path, &json)
            .map_err(|e| SemanticError::Io(format!("{}: {e}", path.display())))
    }

    /// True when this build can use the index for the vault at `location`.
    /// A different `vault_path` means two vault paths share a key.
    pub fn matches(&self, location: &IndexLocation) -> bool {
        self.format_version == FORMAT_VERSION && self.vault_path == location.vault_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_dir_is_keyed_by_the_canonical_vault_path() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = tmp.path().join("vault");
        std::fs::create_dir_all(vault.join("sub")).unwrap();
        let cache = tmp.path().join("cache");

        let location = IndexLocation::for_vault(&cache, &vault).unwrap();
        let canonical = vault.canonicalize().unwrap();
        let key = location.dir.file_name().unwrap().to_string_lossy().into_owned();

        assert_eq!(location.vault_path, canonical);
        assert_eq!(location.dir.parent().unwrap(), cache.join("index"));
        assert_eq!(key.len(), 16);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()), "{key}");
        assert_eq!(key, vault_key(&canonical));
        // Expected value computed independently: sha256("/tmp/vault-137").
        assert_eq!(vault_key(Path::new("/tmp/vault-137")), sha256_prefix("/tmp/vault-137"));

        // Another spelling of the same directory gets the same index.
        let dotted = vault.join("sub").join("..");
        assert_eq!(IndexLocation::for_vault(&cache, &dotted).unwrap(), location);

        let other = tmp.path().join("other");
        std::fs::create_dir_all(&other).unwrap();
        assert_ne!(IndexLocation::for_vault(&cache, &other).unwrap().dir, location.dir);
    }

    fn sha256_prefix(s: &str) -> String {
        Sha256::digest(s.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()[..16]
            .to_string()
    }

    #[test]
    fn key_collision_is_detected_through_vault_path() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = tmp.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        let location = IndexLocation::for_vault(&tmp.path().join("cache"), &vault).unwrap();

        assert!(IndexState::new(location.vault_path.clone()).matches(&location));

        // Another vault whose path hashed to the same key wrote this state.
        let foreign = IndexState::new(PathBuf::from("/somewhere/else/vault-137"));
        assert!(!foreign.matches(&location));

        let mut old_format = IndexState::new(location.vault_path.clone());
        old_format.format_version = FORMAT_VERSION + 1;
        assert!(!old_format.matches(&location));
    }

    #[test]
    fn state_round_trips_through_the_durable_write() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state.json");
        assert_eq!(IndexState::load(&path).unwrap(), None);

        let mut state = IndexState::new(PathBuf::from("/vaults/notes"));
        state.generation = 137;
        state.stale = true;
        state.content_hashes.insert("a1b2c3d4-0000-4000-8000-000000000137".into(), "f00d".into());

        let ((), writes) = crate::fsio::audit::recording(|| state.save(&path).unwrap());
        assert_eq!(writes, vec![path.clone()], "state.json must go through write_atomic");
        assert_eq!(IndexState::load(&path).unwrap(), Some(state.clone()));

        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for field in ["format_version", "generation", "stale", "vault_path", "content_hashes"] {
            assert!(json.get(field).is_some(), "state.json lacks {field}: {json}");
        }

        std::fs::write(&path, "{ not json").unwrap();
        assert!(IndexState::load(&path).is_err());
    }

    use std::fs::{OpenOptions, TryLockError};
    use std::process::{Command, Stdio};

    /// Set in the child process spawned by `lock_is_refused_across_processes`.
    const PROBE_ENV: &str = "CERBO_LOCK_PROBE_PATH";
    /// Child exit code when its lock attempt is refused.
    const PROBE_REFUSED: i32 = 37;

    /// Child side: try to lock the file named by `PROBE_ENV` and report the
    /// outcome through the exit code. A no-op in a normal test run.
    #[test]
    fn lock_probe_child() {
        let Ok(path) = std::env::var(PROBE_ENV) else {
            return;
        };
        let file = OpenOptions::new().read(true).write(true).open(path).unwrap();
        match file.try_lock() {
            Ok(()) => std::process::exit(0),
            Err(TryLockError::WouldBlock) => std::process::exit(PROBE_REFUSED),
            Err(TryLockError::Error(e)) => panic!("lock error: {e}"),
        }
    }

    /// `std::fs::File::lock` (stable since Rust 1.89) is the index's advisory
    /// lock: while one process holds it, another process is refused.
    #[test]
    fn lock_is_refused_across_processes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("LOCK");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .unwrap();

        let probe = || {
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "semantic::tests::lock_probe_child", "--test-threads=1"])
                .env(PROBE_ENV, &path)
                .stdout(Stdio::null())
                .status()
                .unwrap()
                .code()
        };

        file.lock().unwrap();
        assert_eq!(probe(), Some(PROBE_REFUSED), "held lock must refuse another process");

        file.unlock().unwrap();
        assert_eq!(probe(), Some(0), "released lock must be acquirable");
    }
}
