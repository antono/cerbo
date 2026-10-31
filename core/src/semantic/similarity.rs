//! Embeddings inside the index: the lazy refresh (design D6), implicit
//! similarity links (D7) and graph-filtered nearest-neighbour search (D8).
//!
//! Saving a page never runs the model. Commands that load the model anyway
//! first re-embed the pages whose title or `page.md` changed, drop the vectors
//! of vanished pages, and rewrite the similarity links those pages touch.

use super::embed::{content_hash, passage_text, query_text, EmbedProgress, Embedder};
use super::fingerprints::Fingerprints;
use super::graph::OxigraphIndex;
use super::query::{query_prefixes, QueryOutput};
use super::session::{IndexOptions, SemanticIndex};
use super::vectors::{FlatVectors, VectorIndex};
use super::{GraphIndex, SemanticError};
use crate::prefix_map::PrefixMap;
use crate::rdf::{self, NS_SCHEMA};
use crate::VaultContext;
use oxigraph::model::{NamedNode, Term};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

/// Similarity links kept per page (design D7).
pub const SIMILARITY_NEIGHBOURS: usize = 5;
/// Lowest cosine that makes two pages similar (design D7).
pub const SIMILARITY_MIN_SCORE: f32 = 0.80;

/// Pages embedded per model call.
const EMBED_BATCH: usize = 32;
/// Candidates fetched per wanted result, and the widening factor (design D8).
const CANDIDATE_FACTOR: usize = 10;
/// Candidates per `VALUES` block, to keep each filter query small.
const VALUES_CHUNK: usize = 1000;
/// Threads computing similarity links after a large refresh.
const SIMILARITY_THREADS: usize = 8;

/// What an embedding refresh changed.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct EmbeddingRefresh {
    /// Pages embedded because they are new or their content changed.
    pub embedded: usize,
    /// Vectors dropped because their page is gone.
    pub removed: usize,
}

/// One search result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchHit {
    pub uuid: String,
    pub title: String,
    /// Cosine similarity to the query, higher is closer.
    pub score: f32,
}

/// What a search returns and which pages it may return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOptions {
    /// At most this many results.
    pub limit: usize,
    /// Keep only objects with every one of these `rdf:type`s
    /// (`prefix:Class` or an IRI).
    pub types: Vec<String>,
    /// Keep only pages carrying an annotation of every one of these types.
    pub annotated: Vec<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self { limit: 10, types: Vec::new(), annotated: Vec::new() }
    }
}

impl SemanticIndex {
    /// True once `cerbo index --embeddings` has run on this vault: the live
    /// generation has a vector file.
    pub fn has_embeddings(&self) -> bool {
        self.location.vectors_path(self.state.generation).exists()
    }

    /// The live generation's vectors, or an empty table of width `dim`.
    fn load_vectors(&self, dim: usize) -> FlatVectors {
        let path = self.location.vectors_path(self.state.generation);
        match FlatVectors::load(&path) {
            Ok(Some(vectors)) if vectors.dim() == dim => vectors,
            Ok(Some(vectors)) => {
                eprintln!(
                    "Warning: re-embedding every page, the vectors have width {} instead of {dim}",
                    vectors.dim()
                );
                FlatVectors::new(dim)
            }
            Ok(None) => FlatVectors::new(dim),
            Err(e) => {
                eprintln!("Warning: re-embedding every page: {e}");
                FlatVectors::new(dim)
            }
        }
    }

    /// Re-embed every page whose title or `page.md` changed since its last
    /// embedding, drop the vectors of vanished pages, and update the
    /// similarity links (design D6, D7). Creates the vault's embeddings on
    /// first use. Call after a catch-up, so titles and pages are current.
    pub fn refresh_embeddings(
        &mut self,
        embedder: &mut dyn Embedder,
        progress: EmbedProgress<'_>,
    ) -> Result<EmbeddingRefresh, SemanticError> {
        self.refresh(embedder, progress).map(|(report, _)| report)
    }

    fn refresh(
        &mut self,
        embedder: &mut dyn Embedder,
        progress: EmbedProgress<'_>,
    ) -> Result<(EmbeddingRefresh, FlatVectors), SemanticError> {
        let mut vectors = self.load_vectors(embedder.dim());
        let titles = self.titles()?;

        // A page is a live object with a `page.md`; the fingerprint table knows
        // both (slot 2 is `page.md`).
        let mut pages: Vec<String> = self
            .fingerprints
            .iter()
            .filter(|(_, stamp)| stamp[2].is_some())
            .map(|(uuid, _)| uuid.clone())
            .collect();
        pages.sort();

        let mut hashes = BTreeMap::new();
        let mut todo: Vec<(String, String, String)> = Vec::new();
        for uuid in &pages {
            let path = self.objects_dir.join(uuid).join("page.md");
            let body = match std::fs::read_to_string(&path) {
                Ok(body) => body,
                Err(e) => {
                    // Keep what the page had; the next refresh tries again.
                    eprintln!("Warning: {} not embedded: {e}", path.display());
                    if let Some(old) = self.state.content_hashes.get(uuid) {
                        hashes.insert(uuid.clone(), old.clone());
                    }
                    continue;
                }
            };
            let title = titles.get(uuid).map(String::as_str).unwrap_or("");
            let hash = content_hash(title, &body);
            if self.state.content_hashes.get(uuid) == Some(&hash) && vectors.get(uuid).is_some() {
                hashes.insert(uuid.clone(), hash);
            } else {
                todo.push((uuid.clone(), passage_text(title, &body), hash));
            }
        }

        let live: HashSet<&String> = pages.iter().collect();
        let removed: Vec<String> = vectors.uuids().into_iter().filter(|u| !live.contains(u)).collect();
        // Pages a removed page may have been among the closest of: their
        // lists get a replacement. The trash already dropped the links
        // themselves, so the graph no longer names these pages.
        let mut neighbours = BTreeSet::new();
        for uuid in &removed {
            if let Some(v) = vectors.get(uuid) {
                neighbours.extend(vectors.above(v, SIMILARITY_MIN_SCORE, Some(uuid)).into_iter().map(|(n, _)| n));
            }
        }
        for uuid in &removed {
            vectors.remove(uuid);
        }

        let total = todo.len();
        let mut done = 0;
        for batch in todo.chunks(EMBED_BATCH) {
            let texts: Vec<String> = batch.iter().map(|(_, text, _)| text.clone()).collect();
            let embedded = embedder.embed(&texts)?;
            for ((uuid, _, hash), vector) in batch.iter().zip(embedded) {
                vectors.upsert(uuid, &vector);
                hashes.insert(uuid.clone(), hash.clone());
            }
            done += batch.len();
            if let Some(report) = progress {
                report(done, total);
            }
        }

        // Links, then vectors, then hashes: until the hashes are saved, a
        // failure or crash only makes the next refresh embed these pages and
        // drop those vectors again, and so retry their links too.
        let changed: Vec<String> = todo.into_iter().map(|(uuid, _, _)| uuid).collect();
        self.update_similarity(&vectors, &changed, &removed, &neighbours)?;
        vectors.save(&self.location.vectors_path(self.state.generation))?;
        self.state.content_hashes = hashes;
        self.state.save(&self.location.state_path())?;
        Ok((EmbeddingRefresh { embedded: changed.len(), removed: removed.len() }, vectors))
    }

    /// Recompute the similarity links that `changed` and `removed` pages can
    /// affect: those of the changed pages, of pages that linked to a changed or
    /// removed page or are among `neighbours`, and of pages a changed page is
    /// now similar to.
    fn update_similarity(
        &mut self,
        vectors: &FlatVectors,
        changed: &[String],
        removed: &[String],
        neighbours: &BTreeSet<String>,
    ) -> Result<(), SemanticError> {
        if changed.is_empty() && removed.is_empty() {
            return Ok(());
        }
        #[cfg(test)]
        if tests::FAIL_SIMILARITY.with(|fail| fail.replace(false)) {
            return Err(SemanticError::Io("simulated similarity failure".into()));
        }
        let touched: HashSet<&str> = changed.iter().chain(removed).map(String::as_str).collect();
        let mut affected: BTreeSet<String> = changed.iter().cloned().collect();
        affected.extend(neighbours.iter().cloned());
        for (source, target) in self.graph().similarity_edges()? {
            if touched.contains(target.as_str()) {
                affected.insert(source);
            }
        }

        let mut lists = similar_pages(vectors, changed);
        for (_, list) in &lists {
            affected.extend(list.iter().map(|(uuid, _)| uuid.clone()));
        }
        for uuid in removed {
            affected.remove(uuid);
        }
        let changed_set: HashSet<&String> = changed.iter().collect();
        let others: Vec<String> = affected.iter().filter(|u| !changed_set.contains(u)).cloned().collect();
        lists.extend(similar_pages(vectors, &others));

        let edges = edges(lists);
        let sources: Vec<String> = affected.into_iter().chain(removed.iter().cloned()).collect();
        self.graph().replace_similarity(&sources, &edges)
    }

    /// Title of every object, from the graph: the first `schema:name` in sort
    /// order when an object has several.
    fn titles(&self) -> Result<HashMap<String, String>, SemanticError> {
        // Full IRIs: a vault's prefix map may redefine `schema:`.
        let sparql = format!("SELECT ?s ?title WHERE {{ ?s <{NS_SCHEMA}name> ?title }}");
        let QueryOutput::Solutions { rows, .. } = self.live_graph().query(&sparql, &[])? else {
            unreachable!("a SELECT always yields solutions");
        };
        let mut titles: HashMap<String, String> = HashMap::new();
        for row in rows {
            let (Some(Term::NamedNode(s)), Some(Term::Literal(title))) = (&row[0], &row[1]) else {
                continue;
            };
            let Some(uuid) = rdf::uuid_from_object_iri(s.as_str()) else {
                continue;
            };
            let title = title.value();
            titles
                .entry(uuid.to_string())
                .and_modify(|t| {
                    if title < t.as_str() {
                        *t = title.to_string()
                    }
                })
                .or_insert_with(|| title.to_string());
        }
        Ok(titles)
    }

    /// The pages nearest to `query`, best first, keeping only those that pass
    /// the graph filters of `options` and stopping at `options.limit`. The
    /// candidate set widens until enough pages pass or none are left, so a
    /// filter never shrinks the result count (design D8).
    pub fn nearest(
        &self,
        vectors: &FlatVectors,
        query: &[f32],
        options: &SearchOptions,
        exclude: Option<&str>,
    ) -> Result<Vec<SearchHit>, SemanticError> {
        let mut hits = Vec::new();
        if options.limit == 0 {
            return Ok(hits);
        }
        let patterns = self.filter_patterns(options)?;
        let mut k = options.limit.saturating_mul(CANDIDATE_FACTOR);
        let mut checked = 0;
        loop {
            let ranked = vectors.top_k(query, k, exclude);
            for chunk in ranked[checked..].chunks(VALUES_CHUNK) {
                let passing = self.passing_pages(chunk.iter().map(|(uuid, _)| uuid.as_str()), &patterns)?;
                for (uuid, score) in chunk {
                    if let Some(title) = passing.get(uuid) {
                        hits.push(SearchHit { uuid: uuid.clone(), title: title.clone(), score: *score });
                        if hits.len() == options.limit {
                            return Ok(hits);
                        }
                    }
                }
            }
            if ranked.len() < k {
                return Ok(hits);
            }
            checked = ranked.len();
            k = k.saturating_mul(CANDIDATE_FACTOR);
        }
    }

    /// SPARQL patterns on `?page` for the `--type` and `--annotated` filters.
    fn filter_patterns(&self, options: &SearchOptions) -> Result<String, SemanticError> {
        let map = PrefixMap::load(&self.cerbo_dir).unwrap_or_else(|e| {
            eprintln!("Warning: filtering without the vault's prefixes: {e}");
            PrefixMap::default()
        });
        let prefixes = query_prefixes(&map);
        let mut patterns = String::new();
        for class in &options.types {
            let iri = expand(class, &prefixes)?;
            patterns.push_str(&format!("  ?page a <{iri}> .\n"));
        }
        for (i, class) in options.annotated.iter().enumerate() {
            let iri = expand(class, &prefixes)?;
            patterns.push_str(&format!("  ?page <{NS_SCHEMA}mentions> ?a{i} . ?a{i} a <{iri}> .\n"));
        }
        Ok(patterns)
    }

    /// Which of `uuids` match `patterns`, with their titles ("" if none).
    fn passing_pages<'a>(
        &self,
        uuids: impl Iterator<Item = &'a str>,
        patterns: &str,
    ) -> Result<HashMap<String, String>, SemanticError> {
        let values: Vec<String> = uuids.map(|u| format!("<{}>", rdf::object_iri(u).as_str())).collect();
        if values.is_empty() {
            return Ok(HashMap::new());
        }
        let sparql = format!(
            "SELECT ?page ?title WHERE {{\n  VALUES ?page {{ {} }}\n{patterns}  OPTIONAL {{ ?page <{NS_SCHEMA}name> ?title }}\n}}",
            values.join(" ")
        );
        let QueryOutput::Solutions { rows, .. } = self.live_graph().query(&sparql, &[])? else {
            unreachable!("a SELECT always yields solutions");
        };
        let mut passing: HashMap<String, String> = HashMap::new();
        for row in rows {
            let Some(Term::NamedNode(page)) = &row[0] else {
                continue;
            };
            let Some(uuid) = rdf::uuid_from_object_iri(page.as_str()) else {
                continue;
            };
            let title = match &row[1] {
                Some(Term::Literal(l)) => l.value().to_string(),
                _ => String::new(),
            };
            passing
                .entry(uuid.to_string())
                .and_modify(|t| {
                    if t.is_empty() || (!title.is_empty() && title < *t) {
                        *t = title.clone()
                    }
                })
                .or_insert(title);
        }
        Ok(passing)
    }
}

/// The similarity links of `lists`: each page's closest
/// [`SIMILARITY_NEIGHBOURS`], as `(source, target, score)`.
fn edges(lists: Vec<(String, Vec<(String, f32)>)>) -> Vec<(String, String, f32)> {
    lists
        .into_iter()
        .flat_map(|(source, list)| {
            list.into_iter()
                .take(SIMILARITY_NEIGHBOURS)
                .map(move |(target, score)| (source.clone(), target, score))
        })
        .collect()
}

/// Link every page in `live` that has a vector in `vectors_path` to its
/// closest pages, into a `graph` that has no similarity links yet. Used when a
/// rebuild keeps the vectors but lost the store that held the links. A vector
/// file that cannot be read is left for the next refresh to replace.
pub(super) fn link_all(
    graph: &mut OxigraphIndex,
    vectors_path: &Path,
    live: &Fingerprints,
) -> Result<(), SemanticError> {
    let mut vectors = match FlatVectors::load(vectors_path) {
        Ok(Some(vectors)) => vectors,
        Ok(None) => return Ok(()),
        Err(e) => {
            eprintln!("Warning: similarity links not restored: {e}");
            return Ok(());
        }
    };
    for uuid in vectors.uuids() {
        if !live.contains_key(&uuid) {
            vectors.remove(&uuid);
        }
    }
    let pages = vectors.uuids();
    graph.replace_similarity(&[], &edges(similar_pages(&vectors, &pages)))
}

/// Every page each of `uuids` is similar to (score ≥ [`SIMILARITY_MIN_SCORE`]),
/// best first, on [`SIMILARITY_THREADS`] threads. A UUID without a vector gets
/// an empty list, which clears its links.
fn similar_pages(vectors: &FlatVectors, uuids: &[String]) -> Vec<(String, Vec<(String, f32)>)> {
    let list = |uuid: &String| {
        let similar = vectors
            .get(uuid)
            .map(|v| vectors.above(v, SIMILARITY_MIN_SCORE, Some(uuid)))
            .unwrap_or_default();
        (uuid.clone(), similar)
    };
    if uuids.len() < 2 * SIMILARITY_THREADS {
        return uuids.iter().map(list).collect();
    }
    let chunk_size = uuids.len().div_ceil(SIMILARITY_THREADS);
    std::thread::scope(|scope| {
        let workers: Vec<_> = uuids
            .chunks(chunk_size)
            .map(|chunk| scope.spawn(move || chunk.iter().map(list).collect::<Vec<_>>()))
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().expect("similarity worker panicked"))
            .collect()
    })
}

/// The IRI of a filter value: `prefix:Name` with the query prefixes (later
/// entries override earlier ones), `<iri>`, or an absolute IRI.
fn expand(value: &str, prefixes: &[(String, String)]) -> Result<String, SemanticError> {
    let invalid = |why: &str| SemanticError::InvalidFilter(format!("`{value}`: {why}"));
    let iri = if let Some(inner) = value.strip_prefix('<').and_then(|v| v.strip_suffix('>')) {
        inner.to_string()
    } else if value.contains("://") {
        value.to_string()
    } else {
        let (prefix, local) = value.split_once(':').ok_or_else(|| invalid("expected prefix:Name"))?;
        let (_, namespace) = prefixes
            .iter()
            .rev()
            .find(|(p, _)| p == prefix)
            .ok_or_else(|| invalid(&format!("unknown prefix `{prefix}:`")))?;
        format!("{namespace}{local}")
    };
    NamedNode::new(&iri).map_err(|e| invalid(&e.to_string()))?;
    Ok(iri)
}

/// Open the index, catch up with the vault files, and refresh the embeddings:
/// `cerbo index --embeddings`. Creates the vault's embeddings on first use.
pub fn refresh_embeddings(
    vault_ctx: &VaultContext,
    embedder: &mut dyn Embedder,
    options: IndexOptions<'_>,
) -> Result<EmbeddingRefresh, SemanticError> {
    let mut index = SemanticIndex::open(vault_ctx, options)?;
    index.catch_up(options.progress)?;
    index.refresh_embeddings(embedder, options.embed_progress)
}

/// Open the index, catch up, and refresh the embeddings of a vault that has
/// them; [`SemanticError::NoEmbeddings`] otherwise.
fn open_refreshed(
    vault_ctx: &VaultContext,
    embedder: &mut dyn Embedder,
    options: IndexOptions<'_>,
) -> Result<(SemanticIndex, FlatVectors), SemanticError> {
    let mut index = SemanticIndex::open(vault_ctx, options)?;
    index.catch_up(options.progress)?;
    if !index.has_embeddings() {
        return Err(SemanticError::NoEmbeddings);
    }
    let (_, vectors) = index.refresh(embedder, options.embed_progress)?;
    Ok((index, vectors))
}

/// The pages closest in meaning to `text`, best first: `cerbo search`.
pub fn search(
    vault_ctx: &VaultContext,
    text: &str,
    search: &SearchOptions,
    embedder: &mut dyn Embedder,
    options: IndexOptions<'_>,
) -> Result<Vec<SearchHit>, SemanticError> {
    let (index, vectors) = open_refreshed(vault_ctx, embedder, options)?;
    let query = embedder
        .embed(&[query_text(text)])?
        .pop()
        .ok_or_else(|| SemanticError::ModelUnavailable("the model returned no vector".into()))?;
    index.nearest(&vectors, &query, search, None)
}

/// The pages closest in meaning to page `uuid`, best first, without the page
/// itself: `cerbo similar`. An id that is not a live object is
/// [`SemanticError::ObjectNotFound`]; an object without a page has no
/// similar pages.
pub fn similar(
    vault_ctx: &VaultContext,
    uuid: &str,
    search: &SearchOptions,
    embedder: &mut dyn Embedder,
    options: IndexOptions<'_>,
) -> Result<Vec<SearchHit>, SemanticError> {
    let not_found = || SemanticError::ObjectNotFound(uuid.to_string());
    let uuid = uuid::Uuid::parse_str(uuid).map_err(|_| not_found())?.hyphenated().to_string();
    if !vault_ctx.object_path(&uuid).is_dir() {
        return Err(not_found());
    }
    let (index, vectors) = open_refreshed(vault_ctx, embedder, options)?;
    match vectors.get(&uuid) {
        Some(query) => index.nearest(&vectors, query, search, Some(&uuid)),
        None => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::embed::MockEmbedder;
    use super::super::{IndexLocation, IndexState};
    use super::*;
    use crate::rdf::{object_iri, schema};
    use crate::CerboContext;
    use oxigraph::model::{Literal, Triple};
    use std::cell::Cell;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    thread_local! {
        /// Makes the next similarity update on this thread fail.
        pub(super) static FAIL_SIMILARITY: Cell<bool> = const { Cell::new(false) };
    }

    /// A vault whose objects are written as plain files, plus a private cache.
    struct Vault {
        _tmp: TempDir,
        ctx: VaultContext,
        next: usize,
    }

    impl Vault {
        fn new() -> Self {
            let tmp = TempDir::new().unwrap();
            let vault_path = tmp.path().join("vault");
            fs::create_dir_all(vault_path.join(".cerbo").join("objects")).unwrap();
            let ctx = VaultContext {
                vault_path,
                global: CerboContext { config_dir: tmp.path().join("config"), cache_dir: tmp.path().join("cache") },
            };
            Self { _tmp: tmp, ctx, next: 0 }
        }

        /// Write a page of `class` with `body`; `place` adds a `schema:Place`
        /// annotation. UUIDs are sequential, so pages sort by creation.
        fn add(&mut self, class: &str, title: &str, body: &str, place: bool) -> String {
            self.next += 1;
            let uuid = format!("00000000-0000-4000-8000-{:012}", self.next);
            let dir = self.ctx.object_path(&uuid);
            fs::create_dir_all(&dir).unwrap();
            let meta = rdf::serialize(&[
                Triple::new(object_iri(&uuid), rdf::rdf_type(), Term::from(schema(class))),
                Triple::new(object_iri(&uuid), schema("name"), Term::from(Literal::new_simple_literal(title))),
            ]);
            fs::write(dir.join("meta.ttl"), meta).unwrap();
            fs::write(dir.join("page.md"), body).unwrap();
            if place {
                let annotation = format!(
                    "@prefix schema: <https://schema.org/> .\n\
                     <cerbo://objects/{uuid}> schema:mentions [ a schema:Place ; schema:name \"Berlin\" ] .\n"
                );
                fs::write(dir.join("annotations.ttl"), annotation).unwrap();
            }
            uuid
        }

        fn page(&mut self, title: &str, body: &str) -> String {
            self.add("NoteDigitalDocument", title, body, false)
        }

        fn refresh(&self, mock: &mut MockEmbedder) -> EmbeddingRefresh {
            refresh_embeddings(&self.ctx, mock, IndexOptions::default()).unwrap()
        }

        fn location(&self) -> IndexLocation {
            IndexLocation::for_vault(&self.ctx.global.cache_dir, &self.ctx.vault_path).unwrap()
        }

        fn pairs(&self, sparql: &str) -> Vec<(String, String)> {
            let QueryOutput::Solutions { rows, .. } = super::super::query(&self.ctx, sparql, IndexOptions::default()).unwrap()
            else {
                panic!("expected solutions");
            };
            let name = |t: &Option<Term>| match t {
                Some(Term::NamedNode(n)) => rdf::uuid_from_object_iri(n.as_str()).unwrap_or(n.as_str()).to_string(),
                Some(Term::Literal(l)) => l.value().to_string(),
                other => format!("{other:?}"),
            };
            let mut pairs: Vec<(String, String)> = rows.iter().map(|r| (name(&r[0]), name(&r[1]))).collect();
            pairs.sort();
            pairs
        }
    }

    fn uuids(hits: &[SearchHit]) -> Vec<&str> {
        hits.iter().map(|h| h.uuid.as_str()).collect()
    }

    /// Every file under `dir` whose bytes contain `needle`.
    fn files_containing(dir: &Path, needle: &str) -> Vec<PathBuf> {
        walkdir::WalkDir::new(dir)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .filter(|e| String::from_utf8_lossy(&fs::read(e.path()).unwrap()).contains(needle))
            .map(|e| e.path().to_path_buf())
            .collect()
    }

    // ── 5.3 lazy refresh ───────────────────────────────────────

    #[test]
    fn second_refresh_embeds_only_the_edited_page() {
        let mut vault = Vault::new();
        let berlin = vault.page("Berlin", "museums");
        vault.page("Bread", "sourdough");
        let gone = vault.page("Draft", "to be deleted");
        let mut mock = MockEmbedder::default();

        assert_eq!(vault.refresh(&mut mock), EmbeddingRefresh { embedded: 3, removed: 0 });
        assert_eq!(mock.embedded.len(), 3);
        assert!(mock.embedded.iter().all(|t| t.starts_with("passage: ")), "{:?}", mock.embedded);

        // Nothing changed: nothing is embedded.
        assert_eq!(vault.refresh(&mut mock), EmbeddingRefresh::default());

        fs::write(vault.ctx.object_path(&berlin).join("page.md"), "museums and the Spree").unwrap();
        fs::remove_dir_all(vault.ctx.object_path(&gone)).unwrap();
        assert_eq!(vault.refresh(&mut mock), EmbeddingRefresh { embedded: 1, removed: 1 });
        assert_eq!(mock.embedded.len(), 4);
        assert_eq!(mock.embedded[3], "passage: Berlin\nmuseums and the Spree");

        let state = IndexState::load(&vault.location().state_path()).unwrap().unwrap();
        assert_eq!(state.content_hashes.len(), 2, "{:?}", state.content_hashes);
        assert_eq!(state.content_hashes[&berlin], content_hash("Berlin", "museums and the Spree"));
    }

    #[test]
    fn saving_a_page_does_not_embed_but_the_next_search_sees_it() {
        let mut vault = Vault::new();
        vault.page("Bread", "sourdough starter");
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);
        let before = mock.embedded.len();

        let fresh = vault.page("Berlin", "trip to Berlin");
        assert_eq!(mock.embedded.len(), before, "a save must not embed");

        let hits = search(&vault.ctx, "Berlin", &SearchOptions::default(), &mut mock, IndexOptions::default()).unwrap();
        assert_eq!(hits.first().map(|h| h.uuid.as_str()), Some(fresh.as_str()));
        assert_eq!(hits[0].title, "Berlin");
        assert_eq!(mock.embedded.len(), before + 2, "the new page and the query");
        assert_eq!(mock.embedded.last().unwrap(), "query: Berlin");
    }

    #[test]
    fn search_before_embeddings_exist_asks_for_index_embeddings() {
        let mut vault = Vault::new();
        let page = vault.page("Berlin", "trip");
        let mut mock = MockEmbedder::default();

        let err = search(&vault.ctx, "trip", &SearchOptions::default(), &mut mock, IndexOptions::default()).unwrap_err();
        assert_eq!(err, SemanticError::NoEmbeddings);
        assert!(err.to_string().contains("cerbo index --embeddings"), "{err}");
        let err = similar(&vault.ctx, &page, &SearchOptions::default(), &mut mock, IndexOptions::default()).unwrap_err();
        assert_eq!(err, SemanticError::NoEmbeddings);
        assert!(mock.embedded.is_empty(), "nothing may be embedded before opting in");
    }

    #[test]
    fn embeddings_survive_a_rebuild() {
        let mut vault = Vault::new();
        let a = vault.page("Berlin", "Berlin museums and the Spree");
        let b = vault.page("Berlin", "Berlin museums and the Spree river");
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);

        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        let generation = index.state().generation;
        index.rebuild(None).unwrap();
        assert!(index.has_embeddings());
        assert!(!vault.location().vectors_path(generation).exists(), "old vectors left behind");
        drop(index);

        assert_eq!(vault.refresh(&mut mock), EmbeddingRefresh::default(), "a rebuild must not force re-embedding");
        let pairs = vault.pairs("SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }");
        assert_eq!(pairs, [(a.clone(), b.clone()), (b, a)]);
    }

    #[test]
    fn embeddings_survive_lost_fingerprints() {
        let mut vault = Vault::new();
        let a = vault.page("Berlin", "Berlin museums and the Spree");
        let b = vault.page("Berlin", "Berlin museums and the Spree river");
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);
        let generation = IndexState::load(&vault.location().state_path()).unwrap().unwrap().generation;
        fs::remove_file(vault.location().fingerprints_path(generation)).unwrap();

        assert_eq!(vault.refresh(&mut mock), EmbeddingRefresh::default(), "embeddings were dropped");
        let state = IndexState::load(&vault.location().state_path()).unwrap().unwrap();
        assert_eq!(state.generation, generation + 1, "no rebuild happened");
        assert_eq!(state.content_hashes.len(), 2);
        assert_eq!(vault.pairs("SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }"), [(a.clone(), b.clone()), (b, a)]);
    }

    #[test]
    fn embeddings_and_links_survive_an_unreadable_store() {
        let mut vault = Vault::new();
        let a = vault.page("Berlin", "Berlin museums and the Spree");
        let b = vault.page("Berlin", "Berlin museums and the Spree river");
        vault.page("Sourdough", "Feed the starter 137 g of flour");
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);
        let generation = IndexState::load(&vault.location().state_path()).unwrap().unwrap().generation;
        let graph = vault.location().graph_dir(generation);
        fs::remove_dir_all(&graph).unwrap();
        fs::create_dir_all(&graph).unwrap();
        fs::write(graph.join("CURRENT"), "MANIFEST-000137\n").unwrap();

        assert_eq!(vault.refresh(&mut mock), EmbeddingRefresh::default(), "embeddings were dropped");
        assert_eq!(vault.pairs("SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }"), [(a.clone(), b.clone()), (b, a)]);
        assert_eq!(vault.pairs("SELECT ?s ?t WHERE { ?s cerbo:target ?t }").len(), 2);
    }

    #[test]
    fn failed_similarity_update_is_retried_by_the_next_refresh() {
        let mut vault = Vault::new();
        let a = vault.page("Berlin", "Berlin museums and the Spree");
        let b = vault.page("Berlin", "Berlin museums and the Spree river");
        let c = vault.page("Berlin", "Berlin museums and the Spree bridge");
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);
        let before = IndexState::load(&vault.location().state_path()).unwrap().unwrap().content_hashes;

        fs::write(vault.ctx.object_path(&a).join("page.md"), "Sourdough starter and flour").unwrap();
        FAIL_SIMILARITY.with(|fail| fail.set(true));
        refresh_embeddings(&vault.ctx, &mut mock, IndexOptions::default()).unwrap_err();
        let after = IndexState::load(&vault.location().state_path()).unwrap().unwrap().content_hashes;
        assert_eq!(after, before, "hashes saved although the links were not updated");

        assert_eq!(vault.refresh(&mut mock), EmbeddingRefresh { embedded: 1, removed: 0 });
        assert_eq!(
            vault.pairs("SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }"),
            [(b.clone(), c.clone()), (c, b)]
        );
    }

    // ── 5.4 similarity links ───────────────────────────────────

    #[test]
    fn trashed_page_leaves_the_similarity_graph_before_the_next_refresh() {
        let mut vault = Vault::new();
        let a = vault.page("Berlin", "Berlin museums and the Spree");
        let b = vault.page("Berlin", "Berlin museums and the Spree river");
        let c = vault.page("Berlin", "Berlin museums and the Spree bridge");
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);
        fs::remove_dir_all(vault.ctx.object_path(&c)).unwrap();

        // The catch-up before the query drops C's graph and its links.
        let links = "SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }";
        assert_eq!(vault.pairs(links), [(a.clone(), b.clone()), (b.clone(), a.clone())]);
        assert_eq!(vault.pairs("SELECT ?s ?t WHERE { ?s cerbo:target ?t }").len(), 2, "score nodes left behind");
        let mentions_c = format!("ASK {{ GRAPH ?g {{ {{ <cerbo://objects/{c}> ?p ?o }} UNION {{ ?s ?p <cerbo://objects/{c}> }} }} }}");
        let ask = |sparql: &str| super::super::query(&vault.ctx, sparql, IndexOptions::default()).unwrap();
        assert_eq!(ask(&mentions_c), QueryOutput::Boolean(false));

        // A rebuild does not bring them back.
        SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap().rebuild(None).unwrap();
        assert_eq!(vault.pairs(links), [(a.clone(), b.clone()), (b, a)]);
        assert_eq!(ask(&mentions_c), QueryOutput::Boolean(false));
    }

    #[test]
    fn rebuild_drops_links_of_pages_trashed_since_the_last_catch_up() {
        let mut vault = Vault::new();
        let a = vault.page("Berlin", "Berlin museums and the Spree");
        let b = vault.page("Berlin", "Berlin museums and the Spree river");
        let c = vault.page("Berlin", "Berlin museums and the Spree bridge");
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);
        fs::remove_dir_all(vault.ctx.object_path(&c)).unwrap();

        let mut index = SemanticIndex::open(&vault.ctx, IndexOptions::default()).unwrap();
        index.rebuild(None).unwrap();
        let output = index.query("SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }").unwrap();
        let QueryOutput::Solutions { rows, .. } = output else { panic!("expected solutions") };
        assert_eq!(rows.len(), 2, "links of the trashed page were copied: {rows:?}");
        let mentions_c = format!("ASK {{ GRAPH ?g {{ ?s ?p <cerbo://objects/{c}> }} }}");
        assert_eq!(index.query(&mentions_c).unwrap(), QueryOutput::Boolean(false));
        drop(index);
        assert_eq!(vault.pairs("SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }"), [(a.clone(), b.clone()), (b, a)]);
    }

    #[test]
    fn neighbours_of_a_trashed_page_get_a_replacement_link() {
        let mut vault = Vault::new();
        let pages: Vec<String> = (0..7).map(|i| vault.page("Berlin", &format!("Berlin museums Spree {i}"))).collect();
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);
        fs::remove_dir_all(vault.ctx.object_path(&pages[0])).unwrap();

        vault.refresh(&mut mock);
        for page in &pages[1..] {
            let links = vault.pairs(&format!("SELECT ?p ?b WHERE {{ <cerbo://objects/{page}> cerbo:similarTo ?b . BIND(1 AS ?p) }}"));
            assert_eq!(links.len(), SIMILARITY_NEIGHBOURS, "{page}: {links:?}");
        }
    }


    #[test]
    fn similar_pages_are_linked_in_the_index_only() {
        let mut vault = Vault::new();
        let a = vault.page("Berlin", "Berlin museums and the Spree");
        let b = vault.page("Berlin", "Berlin museums and the Spree river");
        vault.page("Sourdough", "Feed the starter 137 g of flour");
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);

        let pairs = vault.pairs("SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }");
        assert_eq!(pairs, [(a.clone(), b.clone()), (b.clone(), a.clone())]);

        // The score node carries the cosine.
        let scored = vault.pairs(&format!(
            "SELECT ?t ?score WHERE {{ ?s a cerbo:Similarity ; cerbo:source <cerbo://objects/{a}> ; \
             cerbo:target ?t ; cerbo:score ?score }}"
        ));
        assert_eq!(scored.len(), 1);
        assert_eq!(scored[0].0, b);
        let score: f32 = scored[0].1.parse().unwrap();
        assert!(score >= SIMILARITY_MIN_SCORE && score <= 1.0, "{score}");

        assert!(files_containing(&vault.ctx.vault_path, "similarTo").is_empty());
        assert!(files_containing(&vault.ctx.vault_path, "Similarity").is_empty());
    }

    #[test]
    fn similarity_follows_edits_and_deletions() {
        let mut vault = Vault::new();
        let a = vault.page("Berlin", "Berlin museums and the Spree");
        let b = vault.page("Berlin", "Berlin museums and the Spree river");
        let c = vault.page("Berlin", "Berlin museums and the Spree bridge");
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);
        assert_eq!(vault.pairs("SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }").len(), 6);

        // A is rewritten about bread: every link to and from it goes.
        fs::write(vault.ctx.object_path(&a).join("page.md"), "Sourdough starter and flour").unwrap();
        vault.refresh(&mut mock);
        assert_eq!(
            vault.pairs("SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }"),
            [(b.clone(), c.clone()), (c.clone(), b.clone())]
        );
        assert_eq!(vault.pairs("SELECT ?s ?t WHERE { ?s cerbo:target ?t }").len(), 2, "score nodes left behind");

        // C is deleted: B's link to it goes with it.
        fs::remove_dir_all(vault.ctx.object_path(&c)).unwrap();
        vault.refresh(&mut mock);
        assert!(vault.pairs("SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }").is_empty());
        assert!(vault.pairs("SELECT ?s ?p WHERE { GRAPH <cerbo://graphs/similarity> { ?s ?p ?o } }").is_empty());
    }

    #[test]
    fn a_page_keeps_at_most_five_similar_pages() {
        let mut vault = Vault::new();
        let pages: Vec<String> = (0..7).map(|i| vault.page("Berlin", &format!("Berlin museums Spree {i}"))).collect();
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);

        for page in &pages {
            // Each link has its score node.
            let links = vault.pairs(&format!(
                "SELECT ?b ?s WHERE {{ <cerbo://objects/{page}> cerbo:similarTo ?b . \
                 ?n cerbo:source <cerbo://objects/{page}> ; cerbo:target ?b ; cerbo:score ?s }}"
            ));
            assert_eq!(links.len(), SIMILARITY_NEIGHBOURS, "{page}: {links:?}");
        }
    }

    // ── 5.5 graph-filtered kNN ─────────────────────────────────

    /// 137 pages mention "city" once among more and more filler, so page `i`
    /// ranks `i`-th; the 20 `schema:Place` pages are the least similar, so a
    /// filtered search must widen past the first candidates.
    fn city_vault() -> (Vault, Vec<String>, Vec<String>) {
        let mut vault = Vault::new();
        let (mut plain, mut places) = (Vec::new(), Vec::new());
        for i in 0..137 {
            let place = i >= 117;
            let filler = "filler ".repeat(if place { 1000 } else { i });
            let class = if i % 10 == 3 { "WebPage" } else { "NoteDigitalDocument" };
            let uuid = vault.add(class, &format!("Page {i}"), &format!("city {filler}"), place);
            if place { places.push(uuid) } else { plain.push(uuid) }
        }
        (vault, plain, places)
    }

    #[test]
    fn filters_do_not_shrink_the_result_count() {
        let (vault, _, places) = city_vault();
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);

        // Unfiltered, the Place pages rank last: the first 100 candidates
        // (limit × 10) hold none of them.
        let all = SearchOptions { limit: 137, ..SearchOptions::default() };
        let ranked = search(&vault.ctx, "city", &all, &mut mock, IndexOptions::default()).unwrap();
        assert_eq!(ranked.len(), 137);
        assert!(ranked[..100].iter().all(|h| !places.contains(&h.uuid)));

        let options = SearchOptions { limit: 10, annotated: vec!["schema:Place".into()], ..SearchOptions::default() };
        let hits = search(&vault.ctx, "city", &options, &mut mock, IndexOptions::default()).unwrap();

        assert_eq!(hits.len(), 10);
        assert!(hits.iter().all(|h| places.contains(&h.uuid)), "{hits:?}");
        assert!(hits.windows(2).all(|w| w[0].score >= w[1].score), "results must stay in kNN order");
        assert!(hits.iter().all(|h| h.title.starts_with("Page ")));
        let expected: Vec<&str> = ranked.iter().filter(|h| places.contains(&h.uuid)).take(10).map(|h| h.uuid.as_str()).collect();
        assert_eq!(uuids(&hits), expected);
    }

    #[test]
    fn type_filter_limit_and_exhausted_vault() {
        let (vault, _, _) = city_vault();
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);

        let three = SearchOptions { limit: 3, ..SearchOptions::default() };
        assert_eq!(search(&vault.ctx, "city", &three, &mut mock, IndexOptions::default()).unwrap().len(), 3);

        // 14 of 137 pages are web pages: asking for 137 returns all 14.
        let web = SearchOptions { limit: 137, types: vec!["schema:WebPage".into()], ..SearchOptions::default() };
        let hits = search(&vault.ctx, "city", &web, &mut mock, IndexOptions::default()).unwrap();
        assert_eq!(hits.len(), 14);
        let full_iri = SearchOptions { types: vec!["<https://schema.org/WebPage>".into()], ..web.clone() };
        assert_eq!(search(&vault.ctx, "city", &full_iri, &mut mock, IndexOptions::default()).unwrap(), hits);

        let both = SearchOptions { annotated: vec!["schema:Place".into()], ..web };
        let hits = search(&vault.ctx, "city", &both, &mut mock, IndexOptions::default()).unwrap();
        assert_eq!(hits.len(), 2, "pages 123 and 133 are web pages with a Place");

        let unknown = SearchOptions { types: vec!["nope:Thing".into()], ..SearchOptions::default() };
        let err = search(&vault.ctx, "city", &unknown, &mut mock, IndexOptions::default()).unwrap_err();
        assert!(matches!(err, SemanticError::InvalidFilter(_)), "{err:?}");
        assert!(err.to_string().contains("nope:"), "{err}");
    }

    #[test]
    fn similar_excludes_the_page_and_rejects_unknown_ids() {
        let mut vault = Vault::new();
        let a = vault.page("Berlin", "Berlin museums and the Spree");
        let b = vault.page("Berlin", "Berlin museums and the Spree river");
        let c = vault.page("Sourdough", "Feed the starter");
        let mut mock = MockEmbedder::default();
        vault.refresh(&mut mock);

        let hits = similar(&vault.ctx, &a.to_uppercase(), &SearchOptions::default(), &mut mock, IndexOptions::default())
            .unwrap();
        assert_eq!(uuids(&hits), [b.as_str(), c.as_str()]);
        assert!(hits[0].score > hits[1].score);

        for unknown in ["00000000-0000-0000-0000-000000000000", "not-a-uuid"] {
            let err = similar(&vault.ctx, unknown, &SearchOptions::default(), &mut mock, IndexOptions::default())
                .unwrap_err();
            assert_eq!(err, SemanticError::ObjectNotFound(unknown.into()));
            assert!(err.to_string().contains("Object not found"));
        }
    }
}
