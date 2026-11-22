//! Vocabulary an agent (or a person) can extend: the vault's ontologies, their
//! types, and vault-owned ontologies that accept new types.
//!
//! A vault-owned ontology is an Ontology object in the vault's own namespace
//! `cerbo://vocab/<uuid>/`, marked `cerbo:vaultOwned`, with its prefix in
//! `.cerbo/ontology-map.json`. Its types are `rdfs:Class` declarations in the
//! object's `meta.ttl`, the same shape the vault import writes. Bundled
//! (Schema.org, FOAF) and imported ontologies never accept added types: a
//! vendor vocabulary is not altered quietly.
//!
//! All functions take the vault's page context, whose `config_dir` is the
//! vault's `.cerbo/`.

use crate::object::{object_path, ObjectMeta, ObjectType};
use crate::prefix_map::{self, PrefixMap};
use crate::{rdf, CerboContext};
use oxrdf::{Literal, NamedNode, NamedOrBlankNode, Term, Triple};
use serde_json::{json, Value};
use uuid::Uuid;

const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";

fn rdfs(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{RDFS}{local}"))
}

fn owned_marker() -> NamedNode {
    rdf::cerbo("vaultOwned")
}

fn meta_path(ctx: &CerboContext, uuid: &str) -> std::path::PathBuf {
    object_path(ctx, uuid).join("meta.ttl")
}

fn read_triples(ctx: &CerboContext, uuid: &str) -> Result<Vec<Triple>, String> {
    let path = meta_path(ctx, uuid);
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    rdf::parse(&raw)
}

fn is_vault_owned(triples: &[Triple], uuid: &str) -> bool {
    let subject = rdf::object_iri(uuid);
    triples.iter().any(|t| matches!(&t.subject, NamedOrBlankNode::NamedNode(n) if *n == subject) && t.predicate == owned_marker())
}

fn literal_of(triples: &[Triple], subject: &NamedNode, predicate: &NamedNode) -> Option<String> {
    triples.iter().find_map(|t| match (&t.subject, &t.object) {
        (NamedOrBlankNode::NamedNode(s), Term::Literal(l)) if s == subject && t.predicate == *predicate => {
            Some(l.value().to_string())
        }
        _ => None,
    })
}

fn node_of(triples: &[Triple], subject: &NamedNode, predicate: &NamedNode) -> Option<String> {
    triples.iter().find_map(|t| match (&t.subject, &t.object) {
        (NamedOrBlankNode::NamedNode(s), Term::NamedNode(o)) if s == subject && t.predicate == *predicate => {
            Some(o.as_str().to_string())
        }
        _ => None,
    })
}

/// The declared classes and properties of `triples` inside `namespace`, as
/// `{name, kind, iri, label, comment, parent}`, sorted by name.
fn declared(triples: &[Triple], namespace: &str) -> Vec<Value> {
    let class = rdfs("Class");
    let property = NamedNode::new_unchecked(format!("{}Property", rdf::NS_RDF));
    let mut out: Vec<(String, Value)> = Vec::new();
    for t in triples {
        let (NamedOrBlankNode::NamedNode(subject), Term::NamedNode(kind)) = (&t.subject, &t.object) else { continue };
        if t.predicate != rdf::rdf_type() || (*kind != class && *kind != property) {
            continue;
        }
        let Some(name) = subject.as_str().strip_prefix(namespace) else { continue };
        out.push((
            name.to_string(),
            json!({
                "name": name,
                "kind": if *kind == class { "class" } else { "property" },
                "iri": subject.as_str(),
                "label": literal_of(triples, subject, &rdfs("label")),
                "comment": literal_of(triples, subject, &rdfs("comment")),
                "parent": node_of(triples, subject, &rdfs("subClassOf")),
            }),
        ));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.dedup_by(|a, b| a.0 == b.0);
    out.into_iter().map(|(_, v)| v).collect()
}

/// Every prefix the vault can annotate with: the vault's own map and the
/// built-in table, as `{prefix, namespace, ontology, builtin, vaultOwned}`.
pub fn list(ctx: &CerboContext) -> Result<Value, String> {
    let map = PrefixMap::load(&ctx.config_dir)?;
    let mut rows: Vec<Value> = Vec::new();
    for (prefix, entry) in &map.prefixes {
        let owned = entry
            .ontology
            .as_deref()
            .and_then(|uuid| read_triples(ctx, uuid).ok().map(|t| is_vault_owned(&t, uuid)))
            .unwrap_or(false);
        rows.push(json!({
            "prefix": prefix,
            "namespace": map.namespace(prefix),
            "ontology": entry.ontology,
            "builtin": prefix_map::builtin_namespace(prefix).is_some(),
            "vaultOwned": owned,
        }));
    }
    for (prefix, namespace) in prefix_map::BUILTIN_PREFIXES {
        if !map.prefixes.contains_key(*prefix) {
            rows.push(json!({
                "prefix": prefix, "namespace": namespace, "ontology": null, "builtin": true, "vaultOwned": false,
            }));
        }
    }
    rows.sort_by(|a, b| a["prefix"].as_str().cmp(&b["prefix"].as_str()));
    Ok(Value::Array(rows))
}

/// The declared types and properties of the ontology behind `prefix`. A
/// prefix with no declarations (Schema.org, FOAF) returns an empty list: any
/// name in its namespace can still be used in an annotation.
pub fn types(ctx: &CerboContext, prefix: &str) -> Result<Value, String> {
    let map = PrefixMap::load(&ctx.config_dir)?;
    let namespace = map.namespace(prefix).ok_or_else(|| format!("unknown prefix: {prefix}"))?.to_string();
    let ontology = map.prefixes.get(prefix).and_then(|e| e.ontology.clone());
    let (list, owned) = match &ontology {
        Some(uuid) => {
            let triples = read_triples(ctx, uuid)?;
            (declared(&triples, &namespace), is_vault_owned(&triples, uuid))
        }
        None => (Vec::new(), false),
    };
    Ok(json!({
        "prefix": prefix,
        "namespace": namespace,
        "ontology": ontology,
        "vaultOwned": owned,
        "types": list,
    }))
}

/// Create a vault-owned ontology with `prefix`. The prefix must be free, in
/// the vault's map and in the built-in table.
pub fn create(ctx: &CerboContext, prefix: &str, title: Option<&str>) -> Result<Value, String> {
    let uuid = Uuid::new_v4().to_string();
    let namespace = format!("cerbo://vocab/{uuid}/");
    prefix_map::validate_registration(prefix, &namespace)?;
    let map = PrefixMap::load(&ctx.config_dir)?;
    if map.prefixes.contains_key(prefix) || prefix_map::builtin_namespace(prefix).is_some() {
        return Err(format!("the prefix {prefix:?} is already in use"));
    }
    let title = title.map(str::trim).filter(|t| !t.is_empty()).map_or_else(|| format!("Vocabulary: {prefix}"), str::to_string);

    let dir = object_path(ctx, &uuid);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let now = chrono::Utc::now().to_rfc3339();
    let meta = ObjectMeta {
        object_type: ObjectType::Ontology,
        title: title.clone(),
        created: now.clone(),
        modified: now,
        original_url: None,
        mime_type: None,
        slug: None,
        virtual_path: None,
    };
    let marker = Triple::new(
        rdf::object_iri(&uuid),
        owned_marker(),
        Term::from(Literal::new_simple_literal("true")),
    );
    crate::fsio::write_atomic_str(&dir.join("meta.ttl"), &meta.to_turtle_with(&uuid, &[marker]))
        .map_err(|e| format!("meta.ttl: {e}"))?;
    crate::fsio::write_atomic_str(&dir.join("page.md"), &ontology_page(&title, &namespace, &[]))
        .map_err(|e| format!("page.md: {e}"))?;
    prefix_map::register_prefix(&ctx.config_dir, prefix, &namespace, Some(&uuid))?;
    crate::object::sync_index(ctx, &uuid);
    Ok(json!({"uuid": uuid, "prefix": prefix, "namespace": namespace, "title": title}))
}

/// Add a class to the vault-owned ontology behind `prefix`. `parent` is a name
/// in the same ontology or `prefix:Name`. A class that is already declared is
/// returned unchanged (`"existing": true`).
pub fn add_type(
    ctx: &CerboContext,
    prefix: &str,
    name: &str,
    label: Option<&str>,
    comment: Option<&str>,
    parent: Option<&str>,
) -> Result<Value, String> {
    let map = PrefixMap::load(&ctx.config_dir)?;
    let refuse = |why: &str| {
        format!(
            "{why}: types can only be added to an ontology owned by this vault; create one first (ontology_create) \
             and use its prefix"
        )
    };
    let namespace = map.namespace(prefix).ok_or_else(|| format!("unknown prefix: {prefix}"))?.to_string();
    let uuid = map
        .prefixes
        .get(prefix)
        .and_then(|e| e.ontology.clone())
        .ok_or_else(|| refuse(&format!("{prefix} is a bundled or external vocabulary")))?;
    let mut triples = read_triples(ctx, &uuid)?;
    if !is_vault_owned(&triples, &uuid) {
        return Err(refuse(&format!("{prefix} is bundled or was imported, so it cannot be changed")));
    }
    let valid = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !valid {
        return Err(format!("invalid type name {name:?}: use letters, digits, `_` and `-`, starting with a letter"));
    }
    let iri = NamedNode::new_unchecked(format!("{namespace}{name}"));
    if triples.iter().any(|t| matches!(&t.subject, NamedOrBlankNode::NamedNode(s) if *s == iri) && t.predicate == rdf::rdf_type()) {
        let existing = declared(&triples, &namespace).into_iter().find(|v| v["name"] == name);
        return Ok(json!({"iri": iri.as_str(), "existing": true, "type": existing}));
    }
    let parent_iri = match parent.map(str::trim).filter(|p| !p.is_empty()) {
        None => None,
        Some(p) => {
            let resolved = match p.split_once(':') {
                Some((pp, local)) => map.resolve(pp, local).ok_or_else(|| format!("unknown prefix in parent {p:?}"))?,
                None => format!("{namespace}{p}"),
            };
            Some(NamedNode::new(resolved).map_err(|e| format!("invalid parent {p:?}: {e}"))?)
        }
    };
    let label = label.map(str::trim).filter(|l| !l.is_empty()).unwrap_or(name);
    triples.push(Triple::new(iri.clone(), rdf::rdf_type(), Term::from(rdfs("Class"))));
    triples.push(Triple::new(iri.clone(), rdfs("label"), Term::from(Literal::new_simple_literal(label))));
    if let Some(comment) = comment.map(str::trim).filter(|c| !c.is_empty()) {
        triples.push(Triple::new(iri.clone(), rdfs("comment"), Term::from(Literal::new_simple_literal(comment))));
    }
    if let Some(parent) = parent_iri {
        triples.push(Triple::new(iri.clone(), rdfs("subClassOf"), Term::from(parent)));
    }
    crate::fsio::write_atomic_str(&meta_path(ctx, &uuid), &rdf::serialize(&triples)).map_err(|e| format!("meta.ttl: {e}"))?;

    // The object's page documents what it declares.
    let title = literal_of(&triples, &rdf::object_iri(&uuid), &rdf::schema("name")).unwrap_or_else(|| format!("Vocabulary: {prefix}"));
    let page = ontology_page(&title, &namespace, &declared(&triples, &namespace));
    crate::fsio::write_atomic_str(&object_path(ctx, &uuid).join("page.md"), &page).map_err(|e| format!("page.md: {e}"))?;
    crate::object::sync_index(ctx, &uuid);
    Ok(json!({"iri": iri.as_str(), "existing": false, "prefix": prefix, "name": name}))
}

fn ontology_page(title: &str, namespace: &str, declared: &[Value]) -> String {
    let mut out = format!("# {title}\n\nA vocabulary owned by this vault.\n\nNamespace: `{namespace}`\n\n## Types\n\n");
    if declared.is_empty() {
        out.push_str("None yet.\n");
    }
    for t in declared {
        let name = t["name"].as_str().unwrap_or_default();
        let label = t["label"].as_str().unwrap_or(name);
        out.push_str(&format!("- `{name}` ({label})"));
        if let Some(comment) = t["comment"].as_str() {
            out.push_str(&format!(": {comment}"));
        }
        out.push('\n');
    }
    out
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
    fn create_extend_and_resolve_an_annotation() {
        let (_tmp, ctx) = ctx();
        let made = create(&ctx, "ex", Some("Kitchen")).unwrap();
        let namespace = made["namespace"].as_str().unwrap().to_string();
        let added = add_type(&ctx, "ex", "Recipe", Some("Recipe"), Some("A way to cook something"), None).unwrap();
        assert_eq!(added["existing"], false);
        let map = PrefixMap::load(&ctx.config_dir).unwrap();
        assert_eq!(map.resolve("ex", "Recipe").unwrap(), format!("{namespace}Recipe"));

        let listed = types(&ctx, "ex").unwrap();
        assert_eq!(listed["vaultOwned"], true);
        assert_eq!(listed["types"][0]["name"], "Recipe");
        assert_eq!(listed["types"][0]["comment"], "A way to cook something");
        let page = std::fs::read_to_string(object_path(&ctx, made["uuid"].as_str().unwrap()).join("page.md")).unwrap();
        assert!(page.contains("`Recipe`"), "{page}");
    }

    #[test]
    fn a_duplicate_changes_nothing_and_names_the_existing_type() {
        let (_tmp, ctx) = ctx();
        let made = create(&ctx, "ex", None).unwrap();
        add_type(&ctx, "ex", "Recipe", None, None, None).unwrap();
        let path = meta_path(&ctx, made["uuid"].as_str().unwrap());
        let before = std::fs::read_to_string(&path).unwrap();
        let again = add_type(&ctx, "ex", "Recipe", Some("Other label"), None, None).unwrap();
        assert_eq!(again["existing"], true);
        assert_eq!(again["type"]["name"], "Recipe");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn a_parent_is_recorded() {
        let (_tmp, ctx) = ctx();
        create(&ctx, "ex", None).unwrap();
        add_type(&ctx, "ex", "Dish", None, None, None).unwrap();
        add_type(&ctx, "ex", "Soup", None, None, Some("Dish")).unwrap();
        add_type(&ctx, "ex", "Idea", None, None, Some("schema:CreativeWork")).unwrap();
        let listed = types(&ctx, "ex").unwrap();
        let by_name = |n: &str| listed["types"].as_array().unwrap().iter().find(|t| t["name"] == n).cloned().unwrap();
        assert!(by_name("Soup")["parent"].as_str().unwrap().ends_with("/Dish"));
        assert_eq!(by_name("Idea")["parent"], "https://schema.org/CreativeWork");
        assert!(add_type(&ctx, "ex", "Bad", None, None, Some("nope:X")).is_err());
    }

    #[test]
    fn bundled_prefixes_refuse_new_types_and_say_where_to_add_them() {
        let (_tmp, ctx) = ctx();
        let err = add_type(&ctx, "schema", "Recipe2", None, None, None).unwrap_err();
        assert!(err.contains("ontology_create"), "{err}");
        assert!(add_type(&ctx, "nope", "X", None, None, None).unwrap_err().contains("unknown prefix"));
    }

    #[test]
    fn an_imported_ontology_refuses_new_types() {
        let (_tmp, ctx) = ctx();
        // An ontology object that is registered but not marked as vault-owned.
        let uuid = Uuid::new_v4().to_string();
        let dir = object_path(&ctx, &uuid);
        std::fs::create_dir_all(&dir).unwrap();
        let meta = ObjectMeta {
            object_type: ObjectType::Ontology,
            title: "Imported".into(),
            created: "2026-01-01T00:00:00Z".into(),
            modified: "2026-01-01T00:00:00Z".into(),
            original_url: Some("https://example.org/vocab".into()),
            mime_type: None,
            slug: None,
            virtual_path: None,
        };
        std::fs::write(dir.join("meta.ttl"), meta.to_turtle(&uuid)).unwrap();
        prefix_map::register_prefix(&ctx.config_dir, "imp", "https://example.org/vocab#", Some(&uuid)).unwrap();
        let err = add_type(&ctx, "imp", "Thing", None, None, None).unwrap_err();
        assert!(err.contains("imported"), "{err}");
    }

    #[test]
    fn a_taken_prefix_fails_and_changes_nothing() {
        let (_tmp, ctx) = ctx();
        create(&ctx, "ex", None).unwrap();
        let before = std::fs::read_to_string(ctx.config_dir.join(prefix_map::MAP_FILE)).unwrap();
        assert!(create(&ctx, "ex", None).is_err());
        assert!(create(&ctx, "schema", None).is_err());
        assert!(create(&ctx, "1bad", None).is_err());
        assert_eq!(std::fs::read_to_string(ctx.config_dir.join(prefix_map::MAP_FILE)).unwrap(), before);
    }

    #[test]
    fn names_are_checked() {
        let (_tmp, ctx) = ctx();
        create(&ctx, "ex", None).unwrap();
        for bad in ["", "1x", "a b", "a/b", "é"] {
            assert!(add_type(&ctx, "ex", bad, None, None, None).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn list_shows_builtin_and_vault_prefixes() {
        let (_tmp, ctx) = ctx();
        create(&ctx, "ex", None).unwrap();
        let rows = list(&ctx).unwrap();
        let find = |p: &str| rows.as_array().unwrap().iter().find(|r| r["prefix"] == p).cloned().unwrap();
        assert_eq!(find("ex")["vaultOwned"], true);
        assert_eq!(find("schema")["builtin"], true);
        assert_eq!(find("schema")["vaultOwned"], false);
    }
}
