//! Views: alternative sidebar trees over the vault's pages (spec `vault-views`,
//! design D1-D6). A view is an object of type `View` (`schema:ItemList`) in
//! `.cerbo/objects/<uuid>/`: `meta.ttl` holds its title and one `schema:hasPart`
//! per page it references, `view.toml` holds the tree. A view never moves or
//! copies a page, and it has no `page.md`, so it is never listed as a page.

use std::collections::BTreeSet;
use std::path::PathBuf;

use oxrdf::{Term, Triple};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CerboContext;
use crate::object::{self, ObjectMeta, ObjectType};

/// File name of the tree inside a view's object directory.
pub const VIEW_FILE: &str = "view.toml";
/// Rows one query node shows (design D5); the total is still reported.
pub const QUERY_ROW_CAP: usize = 137;

const HEADER: &str = "# Sidebar view, written by Cerbo. Safe to edit by hand.\n\
# kind = \"folder\" (name, children), \"page\" (uuid) or \"query\" (name, sparql\n\
# selecting ?page).\n\n";

/// One node of a view's tree, in the JSON shape agents send and in `view.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum ViewNode {
    Folder {
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<ViewNode>,
    },
    Page {
        uuid: String,
    },
    Query {
        name: String,
        sparql: String,
    },
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ViewFile {
    node: Vec<ViewNode>,
}

/// A view in the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ViewMeta {
    pub uuid: String,
    pub title: String,
}

/// A node with its pages looked up, as `view_get` returns it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ResolvedNode {
    Folder { name: String, children: Vec<ResolvedNode> },
    /// `missing` is set when no page has this UUID any more (design D4).
    Page { uuid: String, title: String, missing: bool },
    /// `children` are at most [`QUERY_ROW_CAP`] pages; `total` counts them all.
    /// A failed query has an `error` and no children.
    Query { name: String, children: Vec<ResolvedNode>, total: usize, error: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ViewTree {
    pub uuid: String,
    pub title: String,
    pub nodes: Vec<ResolvedNode>,
}

/// Runs a SPARQL SELECT binding `?page` and returns the page UUIDs, in result
/// order. The caller supplies it (it needs the semantic index, which a
/// vault-scoped `CerboContext` cannot open); see [`pages_from_output`].
pub type QueryRunner<'a> = &'a dyn Fn(&str) -> Result<Vec<String>, String>;

/// The page UUIDs a SELECT output binds to `?page`, in row order. Other
/// variables are ignored; a row without a page IRI is skipped.
pub fn pages_from_output(output: &crate::semantic::QueryOutput) -> Result<Vec<String>, String> {
    let crate::semantic::QueryOutput::Solutions { variables, rows } = output else {
        return Err("a view query must be a SELECT".into());
    };
    let column = variables
        .iter()
        .position(|v| v == "page")
        .ok_or("a view query must select ?page")?;
    Ok(rows
        .iter()
        .filter_map(|row| match row.get(column) {
            Some(Some(Term::NamedNode(iri))) => crate::rdf::uuid_from_object_iri(iri.as_str()).map(str::to_string),
            _ => None,
        })
        .collect())
}

fn view_file_path(ctx: &CerboContext, uuid: &str) -> PathBuf {
    object::object_path(ctx, uuid).join(VIEW_FILE)
}

fn parse_uuid(uuid: &str) -> Result<String, String> {
    Uuid::parse_str(uuid)
        .map(|u| u.hyphenated().to_string())
        .map_err(|_| format!("not a view id: {uuid}"))
}

/// A page is a live object with a `page.md`, which a view never has.
fn page_exists(ctx: &CerboContext, uuid: &str) -> bool {
    object::object_path(ctx, uuid).join("page.md").is_file()
}

fn read_tree(ctx: &CerboContext, uuid: &str) -> Result<Vec<ViewNode>, String> {
    let path = view_file_path(ctx, uuid);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file: ViewFile = toml::from_str(&text).map_err(|e| format!("cannot parse {}: {e}", path.display()))?;
    Ok(file.node)
}

/// The view's metadata, or an error when `uuid` is not a view.
fn read_view_meta(ctx: &CerboContext, uuid: &str) -> Result<ObjectMeta, String> {
    let meta_path = object::object_path(ctx, uuid).join("meta.ttl");
    let meta = ObjectMeta::read_from_file(&meta_path).map_err(|e| format!("view not found: {uuid} ({e})"))?;
    if meta.object_type != ObjectType::View {
        return Err(format!("not a view: {uuid}"));
    }
    Ok(meta)
}

// ── Validation ───────────────────────────────────────────────────────────────

fn check_name(name: &str, what: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err(format!("a {what} name must not be empty"));
    }
    if name.contains('/') {
        return Err(format!("a {what} name must not contain '/': {name}"));
    }
    Ok(())
}

/// Check every node before anything is written (spec "Validation on write").
fn validate(ctx: &CerboContext, nodes: &[ViewNode], run: QueryRunner<'_>) -> Result<(), String> {
    for node in nodes {
        match node {
            ViewNode::Folder { name, children } => {
                check_name(name, "folder")?;
                validate(ctx, children, run)?;
            }
            ViewNode::Page { uuid } => {
                let id = Uuid::parse_str(uuid).map_err(|_| format!("not a page id: {uuid}"))?;
                if !page_exists(ctx, &id.hyphenated().to_string()) {
                    return Err(format!("no page has the id {uuid}"));
                }
            }
            ViewNode::Query { name, sparql } => {
                check_name(name, "query")?;
                run(sparql).map_err(|e| format!("query \"{name}\": {e}"))?;
            }
        }
    }
    Ok(())
}

/// Every page a tree references directly, once each, in first-seen order.
fn referenced_pages(nodes: &[ViewNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            ViewNode::Folder { children, .. } => referenced_pages(children, out),
            ViewNode::Page { uuid } => {
                if let Ok(id) = Uuid::parse_str(uuid) {
                    let id = id.hyphenated().to_string();
                    if !out.contains(&id) {
                        out.push(id);
                    }
                }
            }
            ViewNode::Query { .. } => {}
        }
    }
}

// ── Writing ──────────────────────────────────────────────────────────────────

/// Write `view.toml` and `meta.ttl` (title plus one `schema:hasPart` per
/// referenced page) and refresh the index. `view.toml` goes first: a crash in
/// between leaves a stale membership list, never a tree without its view.
fn write_view(ctx: &CerboContext, uuid: &str, meta: &ObjectMeta, nodes: &[ViewNode]) -> Result<(), String> {
    let dir = object::object_path(ctx, uuid);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;

    let body = toml::to_string(&ViewFile { node: nodes.to_vec() }).map_err(|e| format!("cannot encode view: {e}"))?;
    crate::fsio::write_atomic_str(&view_file_path(ctx, uuid), &format!("{HEADER}{body}"))
        .map_err(|e| format!("cannot write {VIEW_FILE}: {e}"))?;

    let mut pages = Vec::new();
    referenced_pages(nodes, &mut pages);
    let subject = crate::rdf::object_iri(uuid);
    let parts: Vec<Triple> = pages
        .iter()
        .map(|page| Triple::new(subject.clone(), crate::rdf::schema("hasPart"), Term::from(crate::rdf::object_iri(page))))
        .collect();
    crate::fsio::write_atomic_str(&dir.join("meta.ttl"), &meta.to_turtle_with(uuid, &parts))
        .map_err(|e| format!("cannot write meta.ttl: {e}"))?;

    object::sync_index(ctx, uuid);
    Ok(())
}

/// Create a view and return its UUID.
pub fn view_create(ctx: &CerboContext, title: &str, nodes: &[ViewNode], run: QueryRunner<'_>) -> Result<String, String> {
    if title.trim().is_empty() {
        return Err("a view title must not be empty".into());
    }
    validate(ctx, nodes, run)?;
    let uuid = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let meta = ObjectMeta {
        object_type: ObjectType::View,
        title: title.trim().to_string(),
        created: now.clone(),
        modified: now,
        original_url: None,
        mime_type: None,
        slug: None,
        virtual_path: None,
    };
    write_view(ctx, &uuid, &meta, nodes)?;
    Ok(uuid)
}

/// Replace a view's whole tree, and its title when `title` is given. A
/// `view.toml` that does not parse is reported and left as it is.
pub fn view_update(
    ctx: &CerboContext,
    uuid: &str,
    title: Option<&str>,
    nodes: &[ViewNode],
    run: QueryRunner<'_>,
) -> Result<(), String> {
    let uuid = parse_uuid(uuid)?;
    let mut meta = read_view_meta(ctx, &uuid)?;
    // Refuse before writing when the stored file is broken (spec "Broken file
    // is not overwritten").
    if view_file_path(ctx, &uuid).exists() {
        read_tree(ctx, &uuid)?;
    }
    if let Some(title) = title {
        if title.trim().is_empty() {
            return Err("a view title must not be empty".into());
        }
        meta.title = title.trim().to_string();
    }
    validate(ctx, nodes, run)?;
    meta.modified = chrono::Utc::now().to_rfc3339();
    write_view(ctx, &uuid, &meta, nodes)
}

/// Move a view to the trash. Pages are not touched.
pub fn view_delete(ctx: &CerboContext, uuid: &str) -> Result<(), String> {
    let uuid = parse_uuid(uuid)?;
    read_view_meta(ctx, &uuid)?;
    object::object_delete(ctx, &uuid)
}

// ── Reading ──────────────────────────────────────────────────────────────────

/// Every view, by title.
pub fn view_list(ctx: &CerboContext) -> Result<Vec<ViewMeta>, String> {
    let dir = object::objects_dir(ctx);
    let mut views = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(views);
    };
    for entry in entries.flatten() {
        let uuid = entry.file_name().to_string_lossy().to_string();
        if crate::fsio::is_temp_name(&uuid) || !entry.path().is_dir() {
            continue;
        }
        if let Ok(meta) = ObjectMeta::read_from_file(&entry.path().join("meta.ttl"))
            && meta.object_type == ObjectType::View
        {
            views.push(ViewMeta { uuid, title: meta.title });
        }
    }
    views.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()).then(a.uuid.cmp(&b.uuid)));
    Ok(views)
}

fn page_node(ctx: &CerboContext, uuid: &str) -> ResolvedNode {
    if !page_exists(ctx, uuid) {
        return ResolvedNode::Page { uuid: uuid.to_string(), title: String::new(), missing: true };
    }
    let title = ObjectMeta::read_from_file(&object::object_path(ctx, uuid).join("meta.ttl"))
        .map(|m| m.title)
        .unwrap_or_default();
    ResolvedNode::Page { uuid: uuid.to_string(), title, missing: false }
}

fn resolve(ctx: &CerboContext, nodes: &[ViewNode], run: QueryRunner<'_>) -> Vec<ResolvedNode> {
    nodes
        .iter()
        .map(|node| match node {
            ViewNode::Folder { name, children } => {
                ResolvedNode::Folder { name: name.clone(), children: resolve(ctx, children, run) }
            }
            ViewNode::Page { uuid } => {
                let id = Uuid::parse_str(uuid).map(|u| u.hyphenated().to_string()).unwrap_or_else(|_| uuid.clone());
                page_node(ctx, &id)
            }
            ViewNode::Query { name, sparql } => match run(sparql) {
                Ok(found) => {
                    // A page listed twice by the query shows once.
                    let mut seen = BTreeSet::new();
                    let unique: Vec<String> = found.into_iter().filter(|u| seen.insert(u.clone())).collect();
                    let children = unique.iter().take(QUERY_ROW_CAP).map(|u| page_node(ctx, u)).collect();
                    ResolvedNode::Query { name: name.clone(), children, total: unique.len(), error: None }
                }
                Err(e) => ResolvedNode::Query { name: name.clone(), children: Vec::new(), total: 0, error: Some(e) },
            },
        })
        .collect()
}

/// A view's stored nodes, as `view.toml` holds them (for a change that keeps them).
pub fn view_nodes(ctx: &CerboContext, uuid: &str) -> Result<Vec<ViewNode>, String> {
    let uuid = parse_uuid(uuid)?;
    read_view_meta(ctx, &uuid)?;
    read_tree(ctx, &uuid)
}

/// A view's tree with every page looked up and every query node evaluated. A
/// query that fails fails only its node (spec "Query error is local").
pub fn view_get(ctx: &CerboContext, uuid: &str, run: QueryRunner<'_>) -> Result<ViewTree, String> {
    let uuid = parse_uuid(uuid)?;
    let meta = read_view_meta(ctx, &uuid)?;
    let nodes = read_tree(ctx, &uuid)?;
    Ok(ViewTree { uuid, title: meta.title, nodes: resolve(ctx, &nodes, run) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context() -> (tempfile::TempDir, CerboContext) {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        std::fs::create_dir_all(dir.join("objects")).unwrap();
        let ctx = CerboContext { config_dir: dir.clone(), cache_dir: dir.join("cache") };
        (tmp, ctx)
    }

    fn page(ctx: &CerboContext, title: &str) -> String {
        object::object_create(ctx, None, ObjectType::Product, title.into()).unwrap()
    }

    fn no_query(_: &str) -> Result<Vec<String>, String> {
        Err("no queries in this test".into())
    }

    fn page_node_of(uuid: &str) -> ViewNode {
        ViewNode::Page { uuid: uuid.into() }
    }

    #[test]
    fn a_view_is_not_a_page_and_lists_by_title() {
        let (_tmp, ctx) = test_context();
        let p = page(&ctx, "Home");
        let v = view_create(&ctx, "Trip", &[page_node_of(&p)], &no_query).unwrap();
        assert_eq!(crate::page::page_list(&ctx).unwrap().len(), 1);
        assert_eq!(view_list(&ctx).unwrap(), vec![ViewMeta { uuid: v, title: "Trip".into() }]);
    }

    #[test]
    fn the_same_page_may_stand_in_two_folders() {
        let (_tmp, ctx) = test_context();
        let p = page(&ctx, "Home");
        let nodes = vec![
            ViewNode::Folder { name: "Start".into(), children: vec![page_node_of(&p)] },
            ViewNode::Folder { name: "Today".into(), children: vec![page_node_of(&p)] },
        ];
        let v = view_create(&ctx, "Two", &nodes, &no_query).unwrap();
        let tree = view_get(&ctx, &v, &no_query).unwrap();
        let titles: Vec<String> = tree
            .nodes
            .iter()
            .flat_map(|n| match n {
                ResolvedNode::Folder { children, .. } => children.clone(),
                _ => vec![],
            })
            .map(|n| match n {
                ResolvedNode::Page { title, .. } => title,
                _ => String::new(),
            })
            .collect();
        assert_eq!(titles, ["Home", "Home"]);
    }

    #[test]
    fn creation_refuses_an_unknown_page_and_writes_nothing() {
        let (_tmp, ctx) = test_context();
        let ghost = Uuid::new_v4().to_string();
        let err = view_create(&ctx, "Bad", &[page_node_of(&ghost)], &no_query).unwrap_err();
        assert!(err.contains(&ghost), "{err}");
        assert!(view_list(&ctx).unwrap().is_empty());
        assert_eq!(std::fs::read_dir(object::objects_dir(&ctx)).unwrap().count(), 0);
    }

    #[test]
    fn names_and_titles_are_validated() {
        let (_tmp, ctx) = test_context();
        assert!(view_create(&ctx, " ", &[], &no_query).is_err());
        let bad = ViewNode::Folder { name: "a/b".into(), children: vec![] };
        assert!(view_create(&ctx, "T", &[bad], &no_query).unwrap_err().contains("'/'"));
        let empty = ViewNode::Folder { name: "".into(), children: vec![] };
        assert!(view_create(&ctx, "T", &[empty], &no_query).is_err());
    }

    #[test]
    fn a_deleted_page_reads_as_missing_and_the_file_is_unchanged() {
        let (_tmp, ctx) = test_context();
        let p = page(&ctx, "Gone");
        let v = view_create(&ctx, "V", &[page_node_of(&p)], &no_query).unwrap();
        let before = std::fs::read_to_string(view_file_path(&ctx, &v)).unwrap();
        crate::page::page_delete(&ctx, p.clone()).unwrap();
        let tree = view_get(&ctx, &v, &no_query).unwrap();
        assert_eq!(tree.nodes, vec![ResolvedNode::Page { uuid: p, title: String::new(), missing: true }]);
        assert_eq!(std::fs::read_to_string(view_file_path(&ctx, &v)).unwrap(), before);
    }

    #[test]
    fn meta_records_has_part_for_direct_pages_only() {
        let (_tmp, ctx) = test_context();
        let a = page(&ctx, "A");
        let b = page(&ctx, "B");
        let nodes = vec![
            page_node_of(&a),
            ViewNode::Folder { name: "F".into(), children: vec![page_node_of(&a), page_node_of(&b)] },
        ];
        let v = view_create(&ctx, "V", &nodes, &no_query).unwrap();
        let ttl = std::fs::read_to_string(object::object_path(&ctx, &v).join("meta.ttl")).unwrap();
        assert_eq!(ttl.matches(&format!("objects/{a}")).count(), 1, "{ttl}");
        assert_eq!(ttl.matches(&format!("objects/{b}")).count(), 1, "{ttl}");
        assert!(ttl.contains("hasPart"), "{ttl}");
        // An update drops a page that left the view.
        view_update(&ctx, &v, None, &[page_node_of(&a)], &no_query).unwrap();
        let ttl = std::fs::read_to_string(object::object_path(&ctx, &v).join("meta.ttl")).unwrap();
        assert!(!ttl.contains(&b), "{ttl}");
        assert_eq!(ObjectMeta::read_from_file(&object::object_path(&ctx, &v).join("meta.ttl")).unwrap().title, "V");
    }

    #[test]
    fn a_broken_view_file_is_reported_and_not_overwritten() {
        let (_tmp, ctx) = test_context();
        let v = view_create(&ctx, "V", &[], &no_query).unwrap();
        let path = view_file_path(&ctx, &v);
        std::fs::write(&path, "node = [").unwrap();
        let err = view_update(&ctx, &v, None, &[], &no_query).unwrap_err();
        assert!(err.contains(VIEW_FILE), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "node = [");
    }

    #[test]
    fn query_nodes_run_cap_and_fail_locally() {
        let (_tmp, ctx) = test_context();
        let p = page(&ctx, "Found");
        let many: Vec<String> = std::iter::repeat_n(p.clone(), 1).chain((0..200).map(|_| Uuid::new_v4().to_string())).collect();
        let run = |q: &str| match q {
            "good" => Ok(many.clone()),
            _ => Err("boom".to_string()),
        };
        let nodes = vec![
            ViewNode::Query { name: "ok".into(), sparql: "good".into() },
            ViewNode::Query { name: "bad".into(), sparql: "bad".into() },
        ];
        // Validation runs every query: a failing one refuses the write.
        assert!(view_create(&ctx, "V", &nodes, &run).unwrap_err().contains("bad"));
        let v = view_create(&ctx, "V", &nodes[..1], &run).unwrap();
        // The file may later hold a query that fails; only its node fails.
        let mut file = std::fs::read_to_string(view_file_path(&ctx, &v)).unwrap();
        file.push_str("\n[[node]]\nkind = \"query\"\nname = \"bad\"\nsparql = \"bad\"\n");
        std::fs::write(view_file_path(&ctx, &v), file).unwrap();
        let tree = view_get(&ctx, &v, &run).unwrap();
        match &tree.nodes[0] {
            ResolvedNode::Query { children, total, error, .. } => {
                assert_eq!((children.len(), *total, error.is_none()), (QUERY_ROW_CAP, 201, true));
            }
            other => panic!("{other:?}"),
        }
        match &tree.nodes[1] {
            ResolvedNode::Query { error, children, .. } => assert!(error.is_some() && children.is_empty()),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn deleting_a_view_moves_it_to_the_trash_and_spares_pages() {
        let (_tmp, ctx) = test_context();
        let p = page(&ctx, "Keep");
        let v = view_create(&ctx, "V", &[page_node_of(&p)], &no_query).unwrap();
        view_delete(&ctx, &v).unwrap();
        assert!(view_list(&ctx).unwrap().is_empty());
        assert_eq!(crate::page::page_list(&ctx).unwrap().len(), 1);
        // A page is not a view.
        assert!(view_delete(&ctx, &p).unwrap_err().contains("not a view"));
    }

    #[test]
    fn pages_from_output_needs_a_page_column() {
        use crate::semantic::QueryOutput;
        let iri = crate::rdf::object_iri("00000000-0000-4000-8000-000000000137");
        let ok = QueryOutput::Solutions {
            variables: vec!["x".into(), "page".into()],
            rows: vec![vec![None, Some(Term::from(iri))], vec![None, None]],
        };
        assert_eq!(pages_from_output(&ok).unwrap(), ["00000000-0000-4000-8000-000000000137"]);
        let none = QueryOutput::Solutions { variables: vec!["x".into()], rows: vec![] };
        assert!(pages_from_output(&none).unwrap_err().contains("?page"));
        assert!(pages_from_output(&QueryOutput::Boolean(true)).unwrap_err().contains("SELECT"));
    }
}
