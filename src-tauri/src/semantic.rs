//! Semantic commands for the desktop panels (change `add-semantic-desktop-ui`):
//! similar notes, the embedding refresh behind "Enable", the prefix map for
//! mentions, and the SPARQL console. Each runs the same core function as its
//! CLI command, off the main thread (design D1), and one at a time through
//! [`IndexGate`].

use crate::{get_context, get_vault_ctx};
use cerbo_core::prefix_map::PrefixMap;
use cerbo_core::semantic::model::{self, ModelFile};
use cerbo_core::semantic::{
    self as core_semantic, Embedder, EmbeddingRefresh, FastEmbedder, IndexOptions, LazyEmbedder,
    QueryOutput, SearchHit, SearchOptions, SemanticError,
};
use cerbo_core::VaultContext;
use serde::Serialize;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tauri::{AppHandle, Emitter, Manager};

/// Event carrying [`SemanticProgress`] while embeddings are computed.
pub const PROGRESS_EVENT: &str = "semantic://progress";
/// Similar notes listed when the caller names no limit (spec: up to 10).
const SIMILAR_DEFAULT_LIMIT: usize = 10;
/// Upper bound on a requested similar-notes limit.
const SIMILAR_MAX_LIMIT: usize = 100;
/// SELECT rows (or CONSTRUCT/DESCRIBE triples) the console receives; the rest
/// is cut off and flagged as truncated (spec `desktop-sparql-console`).
pub const SPARQL_ROW_LIMIT: usize = 1000;

/// Queues this process's index work: the semantic commands, backlinks and the
/// file watcher's updates.
///
/// The index `LOCK` refuses a second handle even within one process and gives
/// up after `LOCK_TIMEOUT`, so without this queue a request made while "Enable"
/// runs (minutes, with the first model download) would fail as if another
/// Cerbo process held the index. Waiting here instead, a request simply takes
/// longer; `IndexBusy` again only means another process.
#[derive(Default)]
pub struct IndexGate {
    turn: Mutex<()>,
    /// Number of the latest `similar_get`; an older one still queued skips its
    /// work, since its page is no longer open (design D2).
    latest_similar: AtomicU64,
}

impl IndexGate {
    /// Wait for this process's other index work to finish.
    pub fn hold(&self) -> MutexGuard<'_, ()> {
        // The guarded data is `()`: a panicked holder leaves nothing broken.
        self.turn.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Register a new similar-notes request; it supersedes every older one.
    fn similar_ticket(&self) -> u64 {
        self.latest_similar.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn is_latest_similar(&self, ticket: u64) -> bool {
        self.latest_similar.load(Ordering::SeqCst) == ticket
    }
}

/// Why a semantic command has no answer. Tagged by `kind` like
/// `BacklinksError`, so the frontend tells each case apart.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SemanticCommandError {
    /// Another Cerbo process held the index longer than the retry limit.
    IndexBusy,
    /// The vault has no embeddings yet: the panel offers "Enable".
    NoEmbeddings,
    /// A newer `similar_get` came in while this one waited for its turn; the
    /// panel ignores this answer.
    Superseded,
    /// The embedding model cannot be downloaded, verified or loaded.
    ModelUnavailable { message: String },
    /// The query is a SPARQL update; the console is read-only.
    ReadOnlyQuery { message: String },
    /// The query does not parse; the message includes line and column.
    QuerySyntax { message: String },
    /// The query parsed but failed while running.
    QueryEvaluation { message: String },
    /// The page is no longer a live object.
    NotFound { message: String },
    /// Anything else, such as no open vault or an index that cannot be opened.
    Unavailable { message: String },
}

impl From<SemanticError> for SemanticCommandError {
    fn from(e: SemanticError) -> Self {
        let message = e.to_string();
        match e {
            SemanticError::IndexBusy => Self::IndexBusy,
            SemanticError::NoEmbeddings => Self::NoEmbeddings,
            SemanticError::ModelUnavailable(_) => Self::ModelUnavailable { message },
            SemanticError::ReadOnlyQuery => Self::ReadOnlyQuery { message },
            SemanticError::QuerySyntax(_) => Self::QuerySyntax { message },
            SemanticError::QueryEvaluation(_) => Self::QueryEvaluation { message },
            SemanticError::ObjectNotFound(_) => Self::NotFound { message },
            _ => Self::Unavailable { message },
        }
    }
}

impl From<String> for SemanticCommandError {
    fn from(message: String) -> Self {
        Self::Unavailable { message }
    }
}

/// A step of [`embeddings_refresh`] (design D1). `total` is 0 when the step
/// has no count, e.g. the similarity links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SemanticProgress {
    pub phase: Phase,
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// Model files fetched so far, on first use only.
    Download,
    /// Pages embedded so far.
    Embed,
    /// Every page is embedded; the similarity links are being updated.
    Similarity,
}

type Report = Arc<dyn Fn(SemanticProgress) + Send + Sync>;

/// The events for one embedding progress callback: the last batch also
/// announces the similarity step that follows it.
fn embed_events(done: usize, total: usize) -> Vec<SemanticProgress> {
    let mut events = vec![SemanticProgress { phase: Phase::Embed, done, total }];
    if done == total {
        events.push(SemanticProgress { phase: Phase::Similarity, done: 0, total: 0 });
    }
    events
}

/// The similar notes of one page. `uuid` echoes the request, so the panel
/// keeps only the answer for the page still open (design D2).
#[derive(Debug, Serialize)]
pub struct SimilarNotes {
    pub uuid: String,
    pub notes: Vec<SearchHit>,
}

/// A console answer (design D4): SELECT and ASK as SPARQL 1.1 Query Results
/// JSON, CONSTRUCT and DESCRIBE as Turtle. `limit` is the row or triple cap
/// that `truncated` refers to, so the console never hard-codes it.
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "form", rename_all = "camelCase")]
pub enum SparqlResult {
    Results { json: serde_json::Value, truncated: bool, limit: usize },
    Graph { turtle: String, truncated: bool, limit: usize },
}

/// The open vault, for the core semantic functions.
fn vault_context(app: &AppHandle) -> Result<VaultContext, SemanticCommandError> {
    let ctx = get_vault_ctx(app)?;
    let vault_path = ctx.config_dir.parent().unwrap_or(&ctx.config_dir).to_path_buf();
    Ok(VaultContext { vault_path, global: get_context(app)? })
}

/// Emit `progress` to every window; a lost event only costs a progress step.
fn reporter(app: &AppHandle) -> Report {
    let app = app.clone();
    Arc::new(move |progress| {
        if let Err(e) = app.emit(PROGRESS_EVENT, progress) {
            eprintln!("Warning: could not report semantic progress: {e}");
        }
    })
}

/// Run `work` off the main thread, so the editor never waits for the index or
/// the model (design D1).
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, SemanticCommandError> + Send + 'static,
) -> Result<T, SemanticCommandError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| SemanticCommandError::Unavailable { message: e.to_string() })?
}

/// The embedder of `cerbo similar` and `cerbo index --embeddings`, loaded on
/// first use, which also reports the one-time model download.
fn embedder(cache_dir: PathBuf, report: Report) -> LazyEmbedder {
    LazyEmbedder::new(model::MODEL_DIM, move || {
        let env_dir = std::env::var_os(model::MODEL_DIR_ENV).filter(|v| !v.is_empty());
        // A model directory from the environment is never downloaded into.
        if env_dir.is_none() {
            download_missing(&model::cached_model_dir(&cache_dir), &*report)?;
        }
        Ok(Box::new(FastEmbedder::load(&cache_dir)?) as Box<dyn Embedder>)
    })
}

/// Fetch the model files missing from `dir`, reporting each finished file.
/// The model loader verifies them all again afterwards.
fn download_missing(dir: &Path, report: &dyn Fn(SemanticProgress)) -> Result<(), SemanticError> {
    let total = model::MODEL_FILES.len();
    let present = model::MODEL_FILES.iter().filter(|f| dir.join(f.path).exists()).count();
    if present == total {
        return Ok(());
    }
    report(SemanticProgress { phase: Phase::Download, done: present, total });
    let done = Cell::new(present);
    let fetch = |file: &ModelFile, out: &mut File| {
        model::http_fetch(file, out)?;
        done.set(done.get() + 1);
        report(SemanticProgress { phase: Phase::Download, done: done.get(), total });
        Ok(())
    };
    model::ensure_cached_model(dir, &model::MODEL_FILES, &fetch)
}

/// The pages closest in meaning to page `uuid`, best first, without the page
/// itself: `cerbo similar`. Changed pages are re-embedded first.
#[tauri::command]
pub async fn similar_get(
    app: AppHandle,
    uuid: String,
    limit: Option<usize>,
) -> Result<SimilarNotes, SemanticCommandError> {
    let ticket = app.state::<IndexGate>().similar_ticket();
    blocking(move || {
        let gate = app.state::<IndexGate>();
        let _turn = gate.hold();
        if !gate.is_latest_similar(ticket) {
            return Err(SemanticCommandError::Superseded);
        }
        let vault_ctx = vault_context(&app)?;
        let report = reporter(&app);
        let embed_report = Arc::clone(&report);
        let on_embed = move |done: usize, total: usize| embed_events(done, total).into_iter().for_each(|p| embed_report(p));
        let options = IndexOptions { embed_progress: Some(&on_embed), ..IndexOptions::default() };
        let search = SearchOptions { limit: similar_limit(limit), ..SearchOptions::default() };
        let mut embedder = embedder(vault_ctx.global.cache_dir.clone(), report);
        let notes = core_semantic::similar(&vault_ctx, &uuid, &search, &mut embedder, options)?;
        Ok(SimilarNotes { uuid, notes })
    })
    .await
}

fn similar_limit(limit: Option<usize>) -> usize {
    limit.unwrap_or(SIMILAR_DEFAULT_LIMIT).clamp(1, SIMILAR_MAX_LIMIT)
}

/// Create or update the vault's embeddings: `cerbo index --embeddings`, with
/// [`PROGRESS_EVENT`]s for the download, embedding and similarity steps.
#[tauri::command]
pub async fn embeddings_refresh(app: AppHandle) -> Result<EmbeddingRefresh, SemanticCommandError> {
    blocking(move || {
        let gate = app.state::<IndexGate>();
        let _turn = gate.hold();
        let vault_ctx = vault_context(&app)?;
        let report = reporter(&app);
        let embed_report = Arc::clone(&report);
        let on_embed = move |done: usize, total: usize| embed_events(done, total).into_iter().for_each(|p| embed_report(p));
        let options = IndexOptions { embed_progress: Some(&on_embed), ..IndexOptions::default() };
        let mut embedder = embedder(vault_ctx.global.cache_dir.clone(), report);
        Ok(core_semantic::refresh_embeddings(&vault_ctx, &mut embedder, options)?)
    })
    .await
}

/// The open vault's prefixes as `prefix → namespace`: `cerbo:`, the built-ins
/// and `ontology-map.json`, a vault entry winning over a built-in (design D3).
#[tauri::command]
pub fn prefix_map_get(app: AppHandle) -> Result<BTreeMap<String, String>, SemanticCommandError> {
    let vault_ctx = vault_context(&app)?;
    Ok(resolved_prefixes(&PrefixMap::load_vault(&vault_ctx)?))
}

fn resolved_prefixes(map: &PrefixMap) -> BTreeMap<String, String> {
    // Later entries override earlier ones, as in a query.
    core_semantic::query_prefixes(map).into_iter().collect()
}

/// Run a read-only query on the open vault: `cerbo sparql`.
#[tauri::command]
pub async fn sparql_query(app: AppHandle, query: String) -> Result<SparqlResult, SemanticCommandError> {
    blocking(move || {
        let gate = app.state::<IndexGate>();
        let _turn = gate.hold();
        let vault_ctx = vault_context(&app)?;
        let output = core_semantic::query(&vault_ctx, &query, IndexOptions::default())?;
        sparql_result(output, SPARQL_ROW_LIMIT)
    })
    .await
}

/// Cut `output` to `limit` rows or triples and encode it for the console.
fn sparql_result(output: QueryOutput, limit: usize) -> Result<SparqlResult, SemanticCommandError> {
    match output {
        QueryOutput::Graph(mut triples) => {
            let truncated = triples.len() > limit;
            triples.truncate(limit);
            Ok(SparqlResult::Graph { turtle: QueryOutput::Graph(triples).to_text(), truncated, limit })
        }
        QueryOutput::Solutions { variables, mut rows } => {
            let truncated = rows.len() > limit;
            rows.truncate(limit);
            let json = results_json(&QueryOutput::Solutions { variables, rows })?;
            Ok(SparqlResult::Results { json, truncated, limit })
        }
        boolean @ QueryOutput::Boolean(_) => {
            Ok(SparqlResult::Results { json: results_json(&boolean)?, truncated: false, limit })
        }
    }
}

fn results_json(output: &QueryOutput) -> Result<serde_json::Value, SemanticCommandError> {
    serde_json::from_str(&output.to_json()).map_err(|e| SemanticCommandError::Unavailable { message: e.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_busy_has_its_own_kind() {
        let error = SemanticCommandError::from(SemanticError::IndexBusy);
        assert_eq!(serde_json::to_value(&error).unwrap(), serde_json::json!({ "kind": "indexBusy" }));
    }

    #[test]
    fn errors_keep_the_core_message() {
        let error = SemanticCommandError::from(SemanticError::QuerySyntax("line 1 column 137".into()));
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({ "kind": "querySyntax", "message": "SPARQL syntax error: line 1 column 137" })
        );
        assert_eq!(SemanticCommandError::from(SemanticError::NoEmbeddings), SemanticCommandError::NoEmbeddings);
        assert!(matches!(
            SemanticCommandError::from(SemanticError::ModelUnavailable("offline".into())),
            SemanticCommandError::ModelUnavailable { .. }
        ));
        assert!(matches!(
            SemanticCommandError::from(SemanticError::ReadOnlyQuery),
            SemanticCommandError::ReadOnlyQuery { .. }
        ));
    }

    #[test]
    fn similar_limit_defaults_to_ten_and_is_bounded() {
        assert_eq!(similar_limit(None), 10);
        assert_eq!(similar_limit(Some(0)), 1);
        assert_eq!(similar_limit(Some(137)), SIMILAR_MAX_LIMIT);
        assert_eq!(similar_limit(Some(5)), 5);
    }

    #[test]
    fn last_embed_batch_announces_similarity_step() {
        assert_eq!(embed_events(32, 137), vec![SemanticProgress { phase: Phase::Embed, done: 32, total: 137 }]);
        let last = embed_events(137, 137);
        assert_eq!(last[1], SemanticProgress { phase: Phase::Similarity, done: 0, total: 0 });
        assert_eq!(
            serde_json::to_value(last[0]).unwrap(),
            serde_json::json!({ "phase": "embed", "done": 137, "total": 137 })
        );
    }

    #[test]
    fn prefixes_include_builtins_and_vault_overrides() {
        let mut map = PrefixMap::default();
        map.register("ex", "https://example.org/137/", None);
        map.register("schema", "http://schema.org/", None);
        let prefixes = resolved_prefixes(&map);
        assert_eq!(prefixes["ex"], "https://example.org/137/");
        assert_eq!(prefixes["schema"], "http://schema.org/");
        assert_eq!(prefixes["foaf"], "http://xmlns.com/foaf/0.1/");
        assert!(prefixes.contains_key("cerbo"));
    }

    #[test]
    fn select_is_cut_to_the_limit_and_flagged() {
        let rows = vec![vec![None]; 3];
        let output = QueryOutput::Solutions { variables: vec!["p".into()], rows };
        let SparqlResult::Results { json, truncated, limit } = sparql_result(output.clone(), 2).unwrap() else {
            panic!("SELECT is a results document");
        };
        assert!(truncated);
        assert_eq!(limit, 2);
        assert_eq!(json["head"]["vars"], serde_json::json!(["p"]));
        assert_eq!(json["results"]["bindings"].as_array().unwrap().len(), 2);

        let SparqlResult::Results { truncated, .. } = sparql_result(output, 3).unwrap() else {
            panic!("SELECT is a results document");
        };
        assert!(!truncated);
    }

    #[test]
    fn ask_and_construct_keep_their_form() {
        let SparqlResult::Results { json, truncated, .. } = sparql_result(QueryOutput::Boolean(true), 2).unwrap() else {
            panic!("ASK is a results document");
        };
        assert_eq!(json["boolean"], serde_json::json!(true));
        assert!(!truncated);

        let graph = sparql_result(QueryOutput::Graph(Vec::new()), 2).unwrap();
        assert!(matches!(graph, SparqlResult::Graph { truncated: false, limit: 2, .. }));
    }

    #[test]
    fn row_limit_reaches_the_console() {
        let result = sparql_result(QueryOutput::Graph(Vec::new()), 137).unwrap();
        assert_eq!(serde_json::to_value(&result).unwrap()["limit"], serde_json::json!(137));
    }

    #[test]
    fn superseded_has_its_own_kind() {
        assert_eq!(
            serde_json::to_value(SemanticCommandError::Superseded).unwrap(),
            serde_json::json!({ "kind": "superseded" })
        );
    }

    #[test]
    fn only_the_latest_similar_request_runs() {
        let gate = IndexGate::default();
        let first = gate.similar_ticket();
        let second = gate.similar_ticket();
        assert!(!gate.is_latest_similar(first));
        assert!(gate.is_latest_similar(second));
    }

    #[test]
    fn index_work_of_one_process_waits_instead_of_failing() {
        let gate = Arc::new(IndexGate::default());
        let turn = gate.hold();
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter = Arc::clone(&gate);
        let handle = std::thread::spawn(move || {
            let _turn = waiter.hold();
            tx.send(()).unwrap();
        });
        // The second job queues while the first holds its turn...
        assert!(rx.recv_timeout(std::time::Duration::from_millis(137)).is_err());
        drop(turn);
        // ...and runs once it ends.
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        handle.join().unwrap();
    }
}
