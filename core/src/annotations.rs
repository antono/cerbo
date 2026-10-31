use crate::prefix_map::PrefixMap;
use crate::{CerboContext, VaultContext, object, rdf};
use oxrdf::{BlankNode, Literal, NamedNode, Term, Triple};
use regex::Regex;
use std::path::Path;

// ── Annotation Extraction ──────────────────────────────────────

/// Annotation extracted from page content
#[derive(Debug, Clone)]
pub struct Annotation {
    pub text: String,
    pub prefix: String,
    pub type_name: String,
    /// 1-based line in `page.md`.
    pub line: usize,
    /// 0-based character column of the annotation's opening `[`.
    pub column: usize,
}

/// Extract HackMD annotations from page content: [Text]{prefix:Type}
pub fn extract_annotations(content: &str) -> Vec<Annotation> {
    let re = Regex::new(r"\[([^\]]+)\]\{([^}]+)\}").unwrap();
    let mut annotations = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for cap in re.captures_iter(line) {
            if let (Some(whole), Some(text_cap), Some(type_cap)) = (cap.get(0), cap.get(1), cap.get(2)) {
                let full_type = type_cap.as_str().trim();
                // Parse "prefix:Type" or just "Type"
                let (prefix, type_name) = if let Some(pos) = full_type.find(':') {
                    let (p, t) = full_type.split_at(pos);
                    (p.trim().to_string(), t[1..].trim().to_string())
                } else {
                    ("".to_string(), full_type.to_string())
                };

                annotations.push(Annotation {
                    text: text_cap.as_str().to_string(),
                    prefix,
                    type_name,
                    line: line_num + 1,
                    // Where the annotation's `[` sits, counted in characters
                    // from 0, so `I visited [Berlin]{…}` is at column 10.
                    column: line[..whole.start()].chars().count(),
                });
            }
        }
    }

    annotations
}

// ── Annotations.ttl Management ────────────────────────────────────

/// Write annotations.ttl for an object (legacy API). `ctx.config_dir` is the
/// vault's `.cerbo/` directory, as for every other `CerboContext` object call.
///
/// Returns one warning per annotation whose prefix could not be resolved.
pub fn annotations_write(ctx: &CerboContext, uuid: &str, annotations: &[Annotation]) -> Result<Vec<String>, String> {
    let map = PrefixMap::load(&ctx.config_dir)?;
    let obj_dir = object::object_path(ctx, uuid);
    annotations_write_to_path(&map, &obj_dir, uuid, annotations)
}

/// Write annotations.ttl (vault-aware), resolving prefixes with this vault's
/// own `.cerbo/ontology-map.json`.
///
/// Returns one warning per annotation whose prefix could not be resolved.
pub fn annotations_write_vault(vault_ctx: &VaultContext, uuid: &str, annotations: &[Annotation]) -> Result<Vec<String>, String> {
    let map = PrefixMap::load_vault(vault_ctx)?;
    let obj_dir = vault_ctx.object_path(uuid);
    annotations_write_to_path(&map, &obj_dir, uuid, annotations)
}

/// The type of an annotation once its prefix has been looked up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationType {
    /// Full type IRI: the prefix's namespace followed by the local name.
    Resolved(String),
    /// The type as written (`prefix:Type`); the prefix is unknown.
    Unresolved(String),
}

/// Resolve an annotation's `prefix:Type` against a vault's prefix map.
pub fn resolve_annotation_type(map: &PrefixMap, ann: &Annotation) -> AnnotationType {
    let written = if ann.prefix.is_empty() {
        ann.type_name.clone()
    } else {
        format!("{}:{}", ann.prefix, ann.type_name)
    };
    match map.resolve(&ann.prefix, &ann.type_name) {
        // A namespace plus a local name can still be no IRI at all (e.g. a space).
        Some(iri) if NamedNode::new(iri.as_str()).is_ok() => AnnotationType::Resolved(iri),
        _ => AnnotationType::Unresolved(written),
    }
}

/// Build the complete `annotations.ttl` document for a page, plus one warning
/// per annotation whose prefix is unknown.
///
/// Each annotation becomes `<page> schema:mentions _:m` with `schema:name`,
/// `rdf:type` (or `cerbo:unresolvedType`) and `cerbo:position "line:column"`.
/// No annotations give a document with no triples, so a stale file is cleared.
/// The output is a pure function of its inputs, so callers may byte-compare it
/// with the file on disk.
pub fn annotations_ttl(map: &PrefixMap, uuid: &str, annotations: &[Annotation]) -> (String, Vec<String>) {
    let subject = rdf::object_iri(uuid);
    let mut triples = Vec::with_capacity(annotations.len() * 4);
    let mut warnings = Vec::new();

    for (i, ann) in annotations.iter().enumerate() {
        // Deterministic blank-node labels keep the document byte-stable.
        let node = BlankNode::new_unchecked(format!("m{i}"));

        triples.push(Triple::new(
            subject.clone(),
            rdf::schema("mentions"),
            Term::from(node.clone()),
        ));
        triples.push(Triple::new(
            node.clone(),
            rdf::schema("name"),
            Term::from(Literal::new_simple_literal(&ann.text)),
        ));
        match resolve_annotation_type(map, ann) {
            AnnotationType::Resolved(iri) => triples.push(Triple::new(
                node.clone(),
                rdf::rdf_type(),
                Term::from(NamedNode::new_unchecked(iri)),
            )),
            AnnotationType::Unresolved(written) => {
                warnings.push(if ann.prefix.is_empty() {
                    format!(
                        "page {}: annotation [{}]{{{}}} at {}:{} has no prefix; kept as cerbo:unresolvedType",
                        uuid, ann.text, written, ann.line, ann.column
                    )
                } else {
                    format!(
                        "page {}: unknown prefix '{}' in annotation [{}]{{{}}} at {}:{}; kept as cerbo:unresolvedType",
                        uuid, ann.prefix, ann.text, written, ann.line, ann.column
                    )
                });
                triples.push(Triple::new(
                    node.clone(),
                    rdf::cerbo("unresolvedType"),
                    Term::from(Literal::new_simple_literal(written)),
                ));
            }
        }
        triples.push(Triple::new(
            node,
            rdf::cerbo("position"),
            Term::from(Literal::new_simple_literal(format!(
                "{}:{}",
                ann.line, ann.column
            ))),
        ));
    }

    (rdf::serialize(&triples), warnings)
}

fn annotations_write_to_path(
    map: &PrefixMap,
    obj_dir: &Path,
    uuid: &str,
    annotations: &[Annotation],
) -> Result<Vec<String>, String> {
    let (document, warnings) = annotations_ttl(map, uuid, annotations);
    crate::fsio::write_atomic_str(&obj_dir.join("annotations.ttl"), &document)
        .map_err(|e| format!("Failed to write annotations.ttl: {}", e))?;
    Ok(warnings)
}

/// Read annotations.ttl (placeholder - full parsing not implemented)
pub fn annotations_read(ctx: &CerboContext, uuid: &str) -> Result<Vec<Annotation>, String> {
    let obj_dir = object::object_path(ctx, uuid);
    let annotations_path = obj_dir.join("annotations.ttl");

    if !annotations_path.exists() {
        return Ok(Vec::new());
    }

    // Full parsing would require proper Turtle parsing
    // For now, return empty vec
    Ok(Vec::new())
}

// ── Integration with Page Write ──────────────────────────────────────

/// Save `page.md` the way the CLI and the editor do: [`object::object_write`]
/// (date, broken links), then `annotations.ttl` from the new content, then the
/// page's graph in the semantic index.
///
/// The annotations file is always rewritten, so removing the last annotation
/// clears it. Ontologies hold raw RDF rather than notes and, as in indexing,
/// get no annotations.
pub fn page_write_with_annotations(ctx: &CerboContext, uuid: &str, content: &str) -> Result<(), String> {
    object::write_page_content(ctx, uuid, content)?;
    let result = write_page_annotations(ctx, uuid, content);
    // The page's graph is updated once, after all of its files, and also when
    // annotations.ttl failed: page.md and meta.ttl are written either way.
    object::sync_index(ctx, uuid);
    result
}

fn write_page_annotations(ctx: &CerboContext, uuid: &str, content: &str) -> Result<(), String> {
    let meta_path = object::object_path(ctx, uuid).join("meta.ttl");
    if meta_path.exists() {
        let meta = object::ObjectMeta::read_from_file(&meta_path)
            .map_err(|e| format!("page.md was written, but reading meta.ttl failed: {e}"))?;
        if meta.object_type == object::ObjectType::Ontology {
            return Ok(());
        }
    }

    let annotations = extract_annotations(content);
    let warnings = annotations_write(ctx, uuid, &annotations)
        .map_err(|e| format!("page.md was written, but updating annotations.ttl failed: {e}"))?;
    for warning in warnings {
        eprintln!("Warning: {}", warning);
    }

    Ok(())
}

#[cfg(test)]
// Fixtures write files directly; the atomic-write rule guards vault code, not setup.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_annotations() {
        let content = "This is [Bob]{schema:Person} and [Alice]{foaf:Person}.";
        let annotations = extract_annotations(content);

        assert_eq!(annotations.len(), 2);

        assert_eq!(annotations[0].text, "Bob");
        assert_eq!(annotations[0].prefix, "schema");
        assert_eq!(annotations[0].type_name, "Person");

        assert_eq!(annotations[1].text, "Alice");
        assert_eq!(annotations[1].prefix, "foaf");
        assert_eq!(annotations[1].type_name, "Person");
    }

    #[test]
    fn test_extract_annotations_no_prefix() {
        let content = "This is [Bob]{Person}.";
        let annotations = extract_annotations(content);

        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].prefix, "");
        assert_eq!(annotations[0].type_name, "Person");
    }

    #[test]
    fn test_extract_annotations_none() {
        let content = "This is plain text.";
        let annotations = extract_annotations(content);
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn column_is_the_opening_bracket_in_characters() {
        let annotations = extract_annotations("I visited [Berlin]{schema:Place}.\nЯ был в [Берлине]{schema:Place}.");
        assert_eq!((annotations[0].line, annotations[0].column), (1, 10));
        assert_eq!((annotations[1].line, annotations[1].column), (2, 8));
    }

    /// A vault in a temp dir with the given `ontology-map.json` and one page
    /// directory. The global config dir points elsewhere on purpose.
    fn vault_with_map(map_json: &str) -> (tempfile::TempDir, VaultContext) {
        let tmp = tempfile::TempDir::new().unwrap();
        let cerbo_dir = tmp.path().join(".cerbo");
        std::fs::create_dir_all(cerbo_dir.join("objects").join(PAGE)).unwrap();
        std::fs::write(cerbo_dir.join("ontology-map.json"), map_json).unwrap();
        let ctx = VaultContext {
            vault_path: tmp.path().to_path_buf(),
            global: crate::CerboContext {
                config_dir: tmp.path().join("global-config"),
                cache_dir: tmp.path().join("global-cache"),
            },
        };
        (tmp, ctx)
    }

    const PAGE: &str = "aaaaaaaa-0000-0000-0000-000000000137";
    const EMPTY_MAP: &str = r#"{"version":2,"prefixes":{}}"#;

    /// Save `content` the way indexing does and return the parsed triples,
    /// the raw document and the warnings.
    fn save(ctx: &VaultContext, content: &str) -> (Vec<Triple>, String, Vec<String>) {
        let warnings = annotations_write_vault(ctx, PAGE, &extract_annotations(content)).unwrap();
        let ttl = std::fs::read_to_string(ctx.object_path(PAGE).join("annotations.ttl")).unwrap();
        let triples = rdf::parse(&ttl).expect("annotations.ttl must be valid Turtle");
        (triples, ttl, warnings)
    }

    /// The blank node the page mentions with this `schema:name`.
    fn mention<'a>(triples: &'a [Triple], name: &str) -> &'a oxrdf::NamedOrBlankNode {
        let node = triples
            .iter()
            .find(|t| {
                t.predicate == rdf::schema("name")
                    && matches!(&t.object, Term::Literal(l) if l.value() == name)
            })
            .map(|t| &t.subject)
            .unwrap_or_else(|| panic!("no mention named {name}"));
        assert!(
            triples.iter().any(|t| t.subject == rdf::object_iri(PAGE).into()
                && t.predicate == rdf::schema("mentions")
                && t.object == Term::from(node.clone())),
            "the page must schema:mentions {name}"
        );
        node
    }

    fn values<'a>(triples: &'a [Triple], node: &oxrdf::NamedOrBlankNode, predicate: &NamedNode) -> Vec<&'a Term> {
        triples
            .iter()
            .filter(|t| &t.subject == node && &t.predicate == predicate)
            .map(|t| &t.object)
            .collect()
    }

    #[test]
    fn basic_annotation_is_a_typed_mention() {
        let (_tmp, ctx) = vault_with_map(EMPTY_MAP);
        let (triples, _, warnings) = save(&ctx, "I visited [Berlin]{schema:Place}.");

        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(triples.len(), 4);
        let node = mention(&triples, "Berlin");
        assert_eq!(
            values(&triples, node, &rdf::rdf_type()),
            [&Term::from(NamedNode::new_unchecked("https://schema.org/Place"))]
        );
        assert_eq!(
            values(&triples, node, &rdf::cerbo("position")),
            [&Term::from(Literal::new_simple_literal("1:10"))]
        );
    }

    #[test]
    fn position_is_line_colon_column() {
        let (_tmp, ctx) = vault_with_map(EMPTY_MAP);
        let (triples, _, _) = save(&ctx, "1\n2\n3\n4\n0123456789[Berlin]{schema:Place}");
        let node = mention(&triples, "Berlin");
        assert_eq!(
            values(&triples, node, &rdf::cerbo("position")),
            [&Term::from(Literal::new_simple_literal("5:10"))]
        );
    }

    #[test]
    fn custom_prefix_uses_the_vault_map() {
        let (_tmp, ctx) = vault_with_map(
            r#"{"version":2,"prefixes":{"ex":{"namespace":"https://example.org/vocab#","ontology":"bbbbbbbb-0000-0000-0000-000000000137"}}}"#,
        );
        let (triples, ttl, _) = save(&ctx, "[Alice]{ex:Person}");
        let node = mention(&triples, "Alice");
        assert_eq!(
            values(&triples, node, &rdf::rdf_type()),
            [&Term::from(NamedNode::new_unchecked("https://example.org/vocab#Person"))]
        );
        assert!(!ttl.contains("bbbbbbbb"), "the ontology UUID is not part of the type IRI:\n{ttl}");
    }

    #[test]
    fn well_known_prefix_needs_no_map_entry() {
        let (_tmp, ctx) = vault_with_map(EMPTY_MAP);
        let (triples, _, warnings) = save(&ctx, "[Alice]{foaf:Person}");
        assert!(warnings.is_empty());
        let node = mention(&triples, "Alice");
        assert_eq!(
            values(&triples, node, &rdf::rdf_type()),
            [&Term::from(NamedNode::new_unchecked("http://xmlns.com/foaf/0.1/Person"))]
        );
    }

    #[test]
    fn global_config_map_is_not_consulted() {
        let (tmp, ctx) = vault_with_map(EMPTY_MAP);
        std::fs::create_dir_all(&ctx.global.config_dir).unwrap();
        std::fs::write(
            ctx.global.config_dir.join("ontology-map.json"),
            r#"{"version":2,"prefixes":{"zz":{"namespace":"https://global.example/#"}}}"#,
        )
        .unwrap();
        let (_, ttl, warnings) = save(&ctx, "[Thing]{zz:Widget}");
        assert!(!ttl.contains("global.example"), "{ttl}");
        assert_eq!(warnings.len(), 1);
        drop(tmp);
    }

    #[test]
    fn unknown_prefix_is_kept_as_unresolved_type_with_a_warning() {
        let (_tmp, ctx) = vault_with_map(EMPTY_MAP);
        let (triples, _, warnings) = save(&ctx, "[Thing]{zz:Widget} and [Berlin]{schema:Place}");

        let node = mention(&triples, "Thing");
        assert!(values(&triples, node, &rdf::rdf_type()).is_empty(), "no invented IRI");
        assert_eq!(
            values(&triples, node, &rdf::cerbo("unresolvedType")),
            [&Term::from(Literal::new_simple_literal("zz:Widget"))]
        );
        assert_eq!(values(&triples, node, &rdf::cerbo("position")).len(), 1);

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("'zz'"), "{}", warnings[0]);

        // The resolvable one next to it is unaffected.
        let berlin = mention(&triples, "Berlin");
        assert_eq!(values(&triples, berlin, &rdf::rdf_type()).len(), 1);
    }

    #[test]
    fn prefix_registered_later_resolves_on_reindex() {
        let (_tmp, ctx) = vault_with_map(EMPTY_MAP);
        let content = "[Thing]{zz:Widget}";
        let (_, before, _) = save(&ctx, content);
        assert!(before.contains("unresolvedType"));

        crate::prefix_map::register_prefix(&ctx.cerbo_dir(), "zz", "https://zz.example/#", None).unwrap();
        let (triples, after, warnings) = save(&ctx, content);
        assert!(warnings.is_empty());
        assert!(!after.contains("unresolvedType"), "{after}");
        let node = mention(&triples, "Thing");
        assert_eq!(
            values(&triples, node, &rdf::rdf_type()),
            [&Term::from(NamedNode::new_unchecked("https://zz.example/#Widget"))]
        );
    }

    #[test]
    fn multiple_annotations_get_one_mention_each() {
        let (_tmp, ctx) = vault_with_map(EMPTY_MAP);
        let (triples, ttl, _) =
            save(&ctx, "[Bob]{schema:Person} met [Alice]{foaf:Person} at [Acme]{schema:Organization}.");

        // One `schema:mentions` from the object plus three predicates per annotation.
        assert_eq!(triples.len(), 12, "{ttl}");
        for predicate in [
            rdf::schema("mentions"),
            rdf::schema("name"),
            rdf::rdf_type(),
            rdf::cerbo("position"),
        ] {
            let count = triples.iter().filter(|t| t.predicate == predicate).count();
            assert_eq!(count, 3, "every annotation keeps its {predicate}:\n{ttl}");
        }
        for name in ["Bob", "Alice", "Acme"] {
            mention(&triples, name);
        }
    }

    #[test]
    fn last_annotation_removed_clears_the_file() {
        let (_tmp, ctx) = vault_with_map(EMPTY_MAP);
        let (before, _, _) = save(&ctx, "[Berlin]{schema:Place}");
        assert!(!before.is_empty());

        let (after, ttl, _) = save(&ctx, "No annotations any more.");
        assert!(
            !after.iter().any(|t| t.predicate == rdf::schema("mentions")),
            "stale mentions left behind:\n{ttl}"
        );
        assert!(after.is_empty(), "{ttl}");
    }

    #[test]
    fn document_is_byte_stable() {
        let map = PrefixMap::default();
        let annotations = extract_annotations("[Berlin]{schema:Place} [Thing]{zz:Widget}");
        assert_eq!(
            annotations_ttl(&map, PAGE, &annotations).0,
            annotations_ttl(&map, PAGE, &annotations).0
        );
    }

    #[test]
    fn no_prefix_is_unresolved() {
        let map = PrefixMap::default();
        let ann = &extract_annotations("[Bob]{Person}")[0];
        assert_eq!(resolve_annotation_type(&map, ann), AnnotationType::Unresolved("Person".into()));
    }

    #[test]
    fn invalid_iri_is_unresolved() {
        let map = PrefixMap::default();
        let ann = &extract_annotations("[Bob]{schema:Big Person}")[0];
        assert_eq!(
            resolve_annotation_type(&map, ann),
            AnnotationType::Unresolved("schema:Big Person".into())
        );
    }

    #[test]
    fn mentions_answer_a_schema_org_only_query() {
        // `?page schema:mentions [ a schema:Place ; schema:name ?name ]`, by hand.
        let (_tmp, ctx) = vault_with_map(EMPTY_MAP);
        let (triples, _, _) = save(&ctx, "I visited [Berlin]{schema:Place}.");
        let place = Term::from(rdf::schema("Place"));
        let hits: Vec<(&oxrdf::NamedOrBlankNode, &Term)> = triples
            .iter()
            .filter(|m| m.predicate == rdf::schema("mentions"))
            .filter_map(|m| {
                let node = match &m.object {
                    Term::BlankNode(b) => oxrdf::NamedOrBlankNode::from(b.clone()),
                    _ => return None,
                };
                let typed = triples
                    .iter()
                    .any(|t| t.subject == node && t.predicate == rdf::rdf_type() && t.object == place);
                let name = triples
                    .iter()
                    .find(|t| t.subject == node && t.predicate == rdf::schema("name"))?;
                typed.then_some((&m.subject, &name.object))
            })
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, &oxrdf::NamedOrBlankNode::from(rdf::object_iri(PAGE)));
        assert_eq!(hits[0].1, &Term::from(Literal::new_simple_literal("Berlin")));
    }
}
