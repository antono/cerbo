//! What one object contributes to the index, and the Oxigraph engine behind
//! [`GraphIndex`] (design D1, D2).

use super::query::{self, QueryOutput};
use super::{GraphIndex, SemanticError};
use crate::rdf;
use oxigraph::model::{GraphNameRef, NamedOrBlankNode, Quad, QuadRef, Term, Triple};
use oxigraph::store::{Store, Transaction};
use std::collections::BTreeSet;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// One object's triples, as stored in its named graph `cerbo://graphs/<uuid>`:
/// its `meta.ttl`, its `annotations.ttl`, and one `schema:citation` per
/// distinct `cerbo://` link in its `page.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectGraph {
    pub uuid: String,
    pub triples: Vec<Triple>,
}

impl ObjectGraph {
    /// Read the object `uuid` under `objects_dir`. A missing file contributes
    /// nothing. A file that cannot be read or parsed is skipped and named in the
    /// returned warnings: one bad file must not hide the rest of the vault.
    pub fn read(objects_dir: &Path, uuid: &str) -> (Self, Vec<String>) {
        let dir = objects_dir.join(uuid);
        let mut warnings = Vec::new();
        let mut triples = Vec::new();

        if let Some(content) = read_optional(&dir.join("meta.ttl"), &mut warnings) {
            match rdf::parse(&content) {
                Ok(parsed) => triples.extend(parsed),
                // A pre-Turtle meta.ttl still has a readable type and title;
                // index those rather than nothing.
                Err(e) => match crate::object::ObjectMeta::read_from_file(&dir.join("meta.ttl")) {
                    Ok(meta) => triples.extend(rdf::parse(&meta.to_turtle(uuid)).unwrap_or_default()),
                    Err(_) => warnings.push(format!("{uuid}/meta.ttl not indexed: {e}")),
                },
            }
        }

        if let Some(content) = read_optional(&dir.join("annotations.ttl"), &mut warnings) {
            match rdf::parse(&content) {
                Ok(parsed) => triples.extend(parsed),
                Err(e) => warnings.push(format!("{uuid}/annotations.ttl not indexed: {e}")),
            }
        }

        if let Some(content) = read_optional(&dir.join("page.md"), &mut warnings) {
            triples.extend(link_targets(&content).iter().map(|target| rdf::citation(uuid, target)));
        }

        (Self { uuid: uuid.to_string(), triples }, warnings)
    }

    /// The triples as quads in the object's named graph.
    pub fn quads(&self) -> impl Iterator<Item = Quad> + '_ {
        let graph = rdf::graph_iri(&self.uuid);
        self.triples
            .iter()
            .map(move |t| t.clone().in_graph(graph.clone()))
    }
}

/// Distinct, normalised UUIDs that `page.md` links to with `cerbo://`, whether
/// or not they exist: a broken link is kept so it can be found as broken.
pub fn link_targets(page: &str) -> BTreeSet<String> {
    crate::links::extract_cerbo_links(page)
        .iter()
        .filter_map(|id| uuid::Uuid::parse_str(id).ok())
        .map(|id| id.hyphenated().to_string())
        .collect()
}

/// Threads that read object files during a build.
const READ_THREADS: usize = 8;
/// How often a build reports read progress, in objects.
const PROGRESS_EVERY: usize = 1000;

/// Read the graphs of `uuids` under `objects_dir` on [`READ_THREADS`] threads,
/// printing file warnings to stderr and reporting `(done, total)`.
pub fn read_objects(
    objects_dir: &Path,
    uuids: &[String],
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Vec<ObjectGraph> {
    let total = uuids.len();
    let done = AtomicUsize::new(0);
    let read_chunk = |chunk: &[String]| -> Vec<ObjectGraph> {
        chunk
            .iter()
            .map(|uuid| {
                let (graph, warnings) = ObjectGraph::read(objects_dir, uuid);
                for warning in warnings {
                    eprintln!("Warning: {warning}");
                }
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n.is_multiple_of(PROGRESS_EVERY) || n == total {
                    progress(n, total);
                }
                graph
            })
            .collect()
    };
    if total == 0 {
        progress(0, 0);
        return Vec::new();
    }
    let chunk_size = total.div_ceil(READ_THREADS);
    std::thread::scope(|scope| {
        let workers: Vec<_> = uuids
            .chunks(chunk_size)
            .map(|chunk| scope.spawn(|| read_chunk(chunk)))
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().expect("object reader panicked"))
            .collect()
    })
}

fn read_optional(path: &Path, warnings: &mut Vec<String>) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => Some(content),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => {
            warnings.push(format!("{} not indexed: {e}", path.display()));
            None
        }
    }
}

/// [`GraphIndex`] on an Oxigraph RocksDB store in one generation directory.
pub struct OxigraphIndex {
    store: Store,
}

impl OxigraphIndex {
    /// Open (or create) the store in `dir`. Only ever called under the index
    /// `LOCK`: `Store::open_read_only` is not used (design D3).
    pub fn open(dir: &Path) -> Result<Self, SemanticError> {
        let store = Store::open(dir).map_err(storage_error)?;
        Ok(Self { store })
    }

    /// Number of triples in the store, over all object graphs.
    pub fn triples(&self) -> Result<u64, SemanticError> {
        self.store.len().map(|n| n as u64).map_err(storage_error)
    }
}

fn storage_error(e: impl std::fmt::Display) -> SemanticError {
    SemanticError::Io(format!("index storage: {e}"))
}

impl GraphIndex for OxigraphIndex {
    fn apply(&mut self, upserts: &[ObjectGraph], removals: &[String]) -> Result<(), SemanticError> {
        let mut tx = self.store.start_transaction().map_err(storage_error)?;
        for uuid in removals {
            tx.remove_named_graph(&rdf::graph_iri(uuid)).map_err(storage_error)?;
            // A gone page is similar to nothing, and nothing is similar to it.
            for quad in similarity_quads(&tx, uuid, true)? {
                tx.remove(&quad);
            }
        }
        for object in upserts {
            let graph = rdf::graph_iri(&object.uuid);
            // Clearing first removes stale triples exactly, with no diffing.
            tx.clear_graph(&graph).map_err(storage_error)?;
            for t in &object.triples {
                tx.insert(QuadRef::new(
                    t.subject.as_ref(),
                    t.predicate.as_ref(),
                    t.object.as_ref(),
                    GraphNameRef::NamedNode(graph.as_ref()),
                ));
            }
        }
        tx.commit().map_err(storage_error)
    }

    fn load_all(&mut self, objects: &[ObjectGraph]) -> Result<u64, SemanticError> {
        // One loader call for everything: one call per object writes RocksDB
        // files per call and grows a large vault's index by gigabytes (D1).
        let mut loader = self.store.bulk_loader();
        loader
            .load_quads(objects.iter().flat_map(ObjectGraph::quads))
            .map_err(storage_error)?;
        loader.commit().map_err(storage_error)?;
        Ok(objects.iter().map(|o| o.triples.len() as u64).sum())
    }

    fn query(&self, sparql: &str, prefixes: &[(String, String)]) -> Result<QueryOutput, SemanticError> {
        query::run(&self.store, sparql, prefixes)
    }

    fn similarity_triples(&self) -> Result<Vec<Triple>, SemanticError> {
        let graph = rdf::similarity_graph_iri();
        self.store
            .quads_for_pattern(None, None, None, Some(GraphNameRef::NamedNode(graph.as_ref())))
            .map(|q| q.map(Triple::from).map_err(storage_error))
            .collect()
    }

    fn similarity_edges(&self) -> Result<Vec<(String, String)>, SemanticError> {
        let graph = rdf::similarity_graph_iri();
        let similar_to = rdf::cerbo("similarTo");
        let mut edges = Vec::new();
        for quad in self.store.quads_for_pattern(
            None,
            Some(similar_to.as_ref()),
            None,
            Some(GraphNameRef::NamedNode(graph.as_ref())),
        ) {
            let quad = quad.map_err(storage_error)?;
            let (NamedOrBlankNode::NamedNode(s), Term::NamedNode(o)) = (&quad.subject, &quad.object) else {
                continue;
            };
            if let (Some(s), Some(o)) = (rdf::uuid_from_object_iri(s.as_str()), rdf::uuid_from_object_iri(o.as_str())) {
                edges.push((s.to_string(), o.to_string()));
            }
        }
        Ok(edges)
    }

    fn replace_similarity(&mut self, sources: &[String], edges: &[(String, String, f32)]) -> Result<(), SemanticError> {
        let graph = rdf::similarity_graph_iri();
        let mut tx = self.store.start_transaction().map_err(storage_error)?;

        let mut stale = Vec::new();
        for source in sources {
            stale.extend(similarity_quads(&tx, source, false)?);
        }
        for quad in &stale {
            tx.remove(quad);
        }
        for (source, target, score) in edges {
            for t in rdf::similarity(source, target, *score) {
                tx.insert(QuadRef::new(
                    t.subject.as_ref(),
                    t.predicate.as_ref(),
                    t.object.as_ref(),
                    GraphNameRef::NamedNode(graph.as_ref()),
                ));
            }
        }
        tx.commit().map_err(storage_error)
    }
}

/// The quads of the similarity graph for the links from page `uuid`, with
/// their score nodes, and with `incoming` also the links to it.
fn similarity_quads(tx: &Transaction<'_>, uuid: &str, incoming: bool) -> Result<Vec<Quad>, SemanticError> {
    let graph = rdf::similarity_graph_iri();
    let in_graph = Some(GraphNameRef::NamedNode(graph.as_ref()));
    let similar_to = rdf::cerbo("similarTo");
    let iri = rdf::object_iri(uuid);
    let mut quads: Vec<Quad> = tx
        .quads_for_pattern(Some(iri.as_ref().into()), Some(similar_to.as_ref()), None, in_graph)
        .collect::<Result<_, _>>()
        .map_err(storage_error)?;
    let mut roles = vec![rdf::cerbo("source")];
    if incoming {
        quads.extend(
            tx.quads_for_pattern(None, Some(similar_to.as_ref()), Some(iri.as_ref().into()), in_graph)
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?,
        );
        roles.push(rdf::cerbo("target"));
    }
    // Each score node names its source and target; drop the whole node.
    for role in &roles {
        let nodes: Vec<Quad> = tx
            .quads_for_pattern(None, Some(role.as_ref()), Some(iri.as_ref().into()), in_graph)
            .collect::<Result<_, _>>()
            .map_err(storage_error)?;
        for node in nodes {
            for quad in tx.quads_for_pattern(Some(node.subject.as_ref()), None, None, in_graph) {
                quads.push(quad.map_err(storage_error)?);
            }
        }
    }
    Ok(quads)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rdf::{object_iri, schema};
    use oxigraph::model::{NamedOrBlankNodeRef, Term};
    use std::fs;

    const A: &str = "a1b2c3d4-0000-4000-8000-000000000137";
    const B: &str = "b1b2c3d4-0000-4000-8000-000000000137";
    const MISSING: &str = "dead0000-0000-4000-8000-000000000137";

    fn write_object(objects: &Path, uuid: &str, title: &str, page: &str, annotations: Option<&str>) {
        let dir = objects.join(uuid);
        fs::create_dir_all(&dir).unwrap();
        let meta = rdf::serialize(&[
            Triple::new(object_iri(uuid), rdf::rdf_type(), Term::from(schema("NoteDigitalDocument"))),
            Triple::new(
                object_iri(uuid),
                schema("name"),
                Term::from(oxigraph::model::Literal::new_simple_literal(title)),
            ),
        ]);
        fs::write(dir.join("meta.ttl"), meta).unwrap();
        fs::write(dir.join("page.md"), page).unwrap();
        match annotations {
            Some(ttl) => fs::write(dir.join("annotations.ttl"), ttl).unwrap(),
            None => {
                let _ = fs::remove_file(dir.join("annotations.ttl"));
            }
        }
    }

    fn place_annotation(uuid: &str) -> String {
        format!(
            "@prefix schema: <https://schema.org/> .\n\
             <cerbo://objects/{uuid}> schema:mentions [ a schema:Place ; schema:name \"Berlin\" ] .\n"
        )
    }

    fn count(index: &OxigraphIndex, pattern: &str) -> usize {
        let sparql = format!("SELECT * WHERE {{ {pattern} }}");
        match index.query(&sparql, &query::query_prefixes(&Default::default())).unwrap() {
            QueryOutput::Solutions { rows, .. } => rows.len(),
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    fn setup() -> (tempfile::TempDir, std::path::PathBuf, OxigraphIndex) {
        let tmp = tempfile::tempdir().unwrap();
        let objects = tmp.path().join("objects");
        fs::create_dir_all(&objects).unwrap();
        let index = OxigraphIndex::open(&tmp.path().join("graph")).unwrap();
        (tmp, objects, index)
    }

    #[test]
    fn insert_puts_metadata_annotations_and_links_in_the_object_graph() {
        let (_tmp, objects, mut index) = setup();
        let page = format!("# A\n\n[B](cerbo://{B}) and again [B](cerbo://objects/{B})\n");
        write_object(&objects, A, "Trip", &page, Some(&place_annotation(A)));
        write_object(&objects, B, "Berlin", "# Berlin\n", None);

        let (graph, warnings) = ObjectGraph::read(&objects, A);
        assert!(warnings.is_empty(), "{warnings:?}");
        index.upsert_object(&graph).unwrap();

        let g = format!("<cerbo://graphs/{A}>");
        assert_eq!(count(&index, &format!("GRAPH {g} {{ <cerbo://objects/{A}> schema:name \"Trip\" }}")), 1);
        assert_eq!(count(&index, &format!("GRAPH {g} {{ ?a a schema:Place }}")), 1);
        // Two links to the same page are one citation.
        assert_eq!(
            count(&index, &format!("GRAPH {g} {{ <cerbo://objects/{A}> schema:citation <cerbo://objects/{B}> }}")),
            1
        );
    }

    #[test]
    fn replace_drops_removed_annotation_and_removed_link() {
        let (_tmp, objects, mut index) = setup();
        write_object(&objects, A, "Trip", &format!("[B](cerbo://{B})"), Some(&place_annotation(A)));
        index.upsert_object(&ObjectGraph::read(&objects, A).0).unwrap();
        assert_eq!(count(&index, "?a a schema:Place"), 1);
        assert_eq!(count(&index, "?s schema:citation ?t"), 1);

        write_object(&objects, A, "Trip", "no links any more", None);
        index.upsert_object(&ObjectGraph::read(&objects, A).0).unwrap();

        assert_eq!(count(&index, "?a a schema:Place"), 0, "removed annotation survived");
        assert_eq!(count(&index, "?s schema:citation ?t"), 0, "removed link survived");
        assert_eq!(count(&index, &format!("<cerbo://objects/{A}> schema:name \"Trip\"")), 1);
    }

    #[test]
    fn remove_drops_the_whole_graph_including_outgoing_links() {
        let (_tmp, objects, mut index) = setup();
        write_object(&objects, A, "Trip", &format!("[B](cerbo://{B})"), Some(&place_annotation(A)));
        write_object(&objects, B, "Berlin", "", None);
        index.upsert_object(&ObjectGraph::read(&objects, A).0).unwrap();
        index.upsert_object(&ObjectGraph::read(&objects, B).0).unwrap();

        index.remove_object(A).unwrap();

        assert_eq!(count(&index, &format!("GRAPH <cerbo://graphs/{A}> {{ ?s ?p ?o }}")), 0);
        assert_eq!(count(&index, "?s schema:citation ?t"), 0);
        assert!(
            !index
                .store
                .contains_named_graph(NamedOrBlankNodeRef::from(rdf::graph_iri(A).as_ref()))
                .unwrap()
        );
        // Other objects are untouched.
        assert_eq!(count(&index, &format!("<cerbo://objects/{B}> schema:name \"Berlin\"")), 1);
    }

    #[test]
    fn broken_link_is_indexed_and_found_as_broken() {
        let (_tmp, objects, mut index) = setup();
        write_object(&objects, A, "Trip", &format!("[gone](cerbo://{MISSING}) [bad](cerbo://not-a-uuid)"), None);
        index.upsert_object(&ObjectGraph::read(&objects, A).0).unwrap();

        assert_eq!(
            count(&index, &format!("<cerbo://objects/{A}> schema:citation <cerbo://objects/{MISSING}>")),
            1
        );
        // A malformed id is not a link at all.
        assert_eq!(count(&index, "?s schema:citation ?t"), 1);
        assert_eq!(count(&index, &format!("<cerbo://objects/{MISSING}> ?p ?o")), 0);
        assert_eq!(
            count(&index, "?a schema:citation ?t FILTER NOT EXISTS { ?t a ?c }"),
            1,
            "the link must be found as broken"
        );
    }

    #[test]
    fn unparsable_annotations_are_skipped_with_a_warning() {
        let (_tmp, objects, mut index) = setup();
        write_object(&objects, A, "Trip", "", Some("this is not turtle ."));
        let (graph, warnings) = ObjectGraph::read(&objects, A);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("annotations.ttl"), "{warnings:?}");
        index.upsert_object(&graph).unwrap();
        assert_eq!(count(&index, &format!("<cerbo://objects/{A}> schema:name \"Trip\"")), 1);
    }

    #[test]
    fn load_all_uses_one_graph_per_object() {
        let (_tmp, objects, mut index) = setup();
        write_object(&objects, A, "Trip", &format!("[B](cerbo://{B})"), None);
        write_object(&objects, B, "Berlin", "", None);
        let graphs = [ObjectGraph::read(&objects, A).0, ObjectGraph::read(&objects, B).0];

        let triples = index.load_all(&graphs).unwrap();

        assert_eq!(triples, 5);
        assert_eq!(index.store.named_graphs().count(), 2);
        assert_eq!(count(&index, "GRAPH ?g { ?s schema:citation ?t }"), 1);
    }
}
