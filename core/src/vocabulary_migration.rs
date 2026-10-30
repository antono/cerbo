//! `cerbo migrate vocabulary`: rewrite a vault's metadata into the standard
//! vocabulary, so old vaults hold what new writes produce.
//!
//! Every reader already understands the legacy spellings ([`crate::rdf::parse`]
//! upgrades them as it reads), so this is never required for a vault to open.
//! It exists so the files themselves — and anything outside Cerbo that reads
//! them — use schema.org.
//!
//! For each live object:
//!
//! - `meta.ttl` is parsed with the tolerant parser and, if any triple used a
//!   legacy spelling, rewritten exactly as a save would write it. Triples Cerbo
//!   does not model (a user's own `rdf:type schema:Recipe`) are kept. A file from
//!   before the Turtle rewrite, which is not Turtle at all, is rewritten from
//!   the fields the legacy line reader recovers.
//! - `annotations.ttl` is regenerated from `page.md` with the vault's prefix
//!   map, which also repairs types that lost their prefix and old positions.
//!
//! Each new document is byte-compared with the file on disk and only written,
//! atomically, when it differs; that is what makes a second run touch nothing.
//! An object that cannot be read is reported by UUID and left exactly as it was.

use crate::annotations::{annotations_ttl, extract_annotations};
use crate::object::{ObjectMeta, ObjectType};
use crate::prefix_map::PrefixMap;
use crate::{VaultContext, rdf};
use std::fs;
use std::path::{Path, PathBuf};

/// Outcome of migrating one vault.
#[derive(Debug, Default, Clone)]
pub struct MigrationReport {
    /// Live objects looked at.
    pub scanned: usize,
    /// Files rewritten — or, on a dry run, that would be.
    pub rewritten: usize,
    /// Those files, in the order they were (or would be) written.
    pub files: Vec<PathBuf>,
    /// Objects left untouched because they could not be read or written.
    pub errors: Vec<ObjectError>,
    /// Non-fatal notes, such as an annotation whose prefix is unknown.
    pub warnings: Vec<String>,
}

/// An object the migration could not process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectError {
    pub uuid: String,
    pub message: String,
}

/// Migrate every live object of one vault. With `dry_run`, report what would
/// change and write nothing.
///
/// Objects under `.cerbo/trash/` are not live and are never visited. A failure
/// that concerns one object lands in [`MigrationReport::errors`] and the run
/// goes on; only a vault-wide failure (an unreadable object directory or prefix
/// map) is returned as `Err`.
pub fn migrate_vocabulary(vault_ctx: &VaultContext, dry_run: bool) -> Result<MigrationReport, String> {
    let map = PrefixMap::load_vault(vault_ctx)?;
    let mut report = MigrationReport::default();

    for (uuid, obj_dir) in live_objects(&vault_ctx.objects_dir())? {
        report.scanned += 1;

        // Every new document is built before any is written, so an object that
        // fails part-way through planning is left entirely untouched.
        let plan = match plan_object(&map, &obj_dir, &uuid, &mut report.warnings) {
            Ok(plan) => plan,
            Err(message) => {
                report.errors.push(ObjectError { uuid, message });
                continue;
            }
        };

        for (path, document) in plan {
            if !dry_run
                && let Err(e) = crate::fsio::write_atomic_str(&path, &document)
            {
                report.errors.push(ObjectError {
                    uuid: uuid.clone(),
                    message: format!("failed to write {}: {}", path.display(), e),
                });
                break;
            }
            report.rewritten += 1;
            report.files.push(path);
        }
    }

    Ok(report)
}

/// The object directories under `objects/`, sorted by UUID so reports are stable.
fn live_objects(objects_dir: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    if !objects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut objects = Vec::new();
    let entries = fs::read_dir(objects_dir)
        .map_err(|e| format!("failed to read {}: {}", objects_dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read {}: {}", objects_dir.display(), e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !entry.path().is_dir() || crate::fsio::is_temp_name(&name) {
            continue;
        }
        objects.push((name, entry.path()));
    }
    objects.sort();
    Ok(objects)
}

/// The files of one object that need rewriting, with their new content.
fn plan_object(
    map: &PrefixMap,
    obj_dir: &Path,
    uuid: &str,
    warnings: &mut Vec<String>,
) -> Result<Vec<(PathBuf, String)>, String> {
    let mut plan = Vec::new();

    let meta_path = obj_dir.join("meta.ttl");
    let meta_on_disk = fs::read_to_string(&meta_path)
        .map_err(|e| format!("failed to read meta.ttl: {}", e))?;
    let (object_type, meta_doc) = migrate_meta(&meta_on_disk, uuid)?;
    if let Some(doc) = meta_doc
        && doc != meta_on_disk
    {
        plan.push((meta_path, doc));
    }

    // Ontologies hold raw RDF rather than notes, and the index never annotates
    // them. A page with no annotations.ttl yet gets one from the next index run;
    // creating it here would be indexing, not migrating. Likewise a file already
    // in the standard form is left alone even if the page changed since it was
    // written: bringing it up to date is the next save's or index run's job.
    let annotations_path = obj_dir.join("annotations.ttl");
    let page_path = obj_dir.join("page.md");
    if object_type != ObjectType::Ontology && annotations_path.exists() && page_path.exists() {
        let on_disk = fs::read_to_string(&annotations_path)
            .map_err(|e| format!("failed to read annotations.ttl: {}", e))?;
        // A file that does not parse cannot be standard: old writers built it by hand.
        let legacy = match rdf::parse_detecting_legacy(&on_disk) {
            Ok((_, legacy)) => legacy,
            Err(_) => true,
        };
        if legacy {
            let page = fs::read_to_string(&page_path)
                .map_err(|e| format!("failed to read page.md: {}", e))?;
            let (doc, notes) = annotations_ttl(map, uuid, &extract_annotations(&page));
            warnings.extend(notes);
            if doc != on_disk {
                plan.push((annotations_path, doc));
            }
        }
    }

    Ok(plan)
}

/// The object's class and, when `meta.ttl` uses a legacy form, its standard
/// replacement. `None` means the file is already standard and stays as it is.
fn migrate_meta(content: &str, uuid: &str) -> Result<(ObjectType, Option<String>), String> {
    match rdf::parse_detecting_legacy(content) {
        Ok((triples, legacy)) => {
            // Refuses an object carrying two of the four classes, as every
            // other reader does: rewriting it would bake in a guess.
            let meta = ObjectMeta::from_triples(&triples, Some(uuid))?;
            // Written exactly as a save writes it — `schema:url` as an IRI, the
            // fields in their fixed order — followed by the triples Cerbo does
            // not model, so the next ordinary save changes nothing but the date.
            let doc = legacy.then(|| {
                meta.to_turtle_with(uuid, &ObjectMeta::unmodelled_triples(&triples, Some(uuid)))
            });
            Ok((meta.object_type, doc))
        }
        // A file from before the Turtle rewrite. Only one that the legacy line
        // reader recognises is rewritten; anything else is reported, because
        // reserialising an empty note over it would lose whatever it holds.
        Err(parse_error) => match ObjectMeta::from_legacy_lines_checked(content, Some(uuid)) {
            Some(meta) => {
                let meta = meta?;
                Ok((meta.object_type, Some(meta.to_turtle(uuid))))
            }
            None => Err(format!("meta.ttl does not parse: {}", parse_error)),
        },
    }
}

#[cfg(test)]
// Fixtures write files directly; the atomic-write rule guards vault code, not setup.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::CerboContext;
    use std::collections::BTreeMap;
    use std::time::SystemTime;
    use tempfile::TempDir;

    const PAGE: &str = "ba079793-b2ff-4ca6-ba88-8764ce6668c3";
    const SCHEMA_ONTOLOGY: &str = "5c1e0137-0000-4000-8000-000000000137";

    /// The legacy `meta.ttl` from `test-vault`, verbatim.
    const LEGACY_META: &str = concat!(
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
        "@prefix schema: <cerbo://ontology/schema/> .\n",
        "@prefix cerbo: <cerbo://ontology/> .\n",
        "<cerbo://objects/ba079793-b2ff-4ca6-ba88-8764ce6668c3> cerbo:type cerbo:Product ;\n",
        "\tcerbo:title \"Index\" ;\n",
        "\tschema:dateCreated \"2026-10-25T13:17:54.772697296+00:00\"^^xsd:dateTime ;\n",
        "\tschema:dateModified \"2026-10-25T13:18:16.608529895+00:00\"^^xsd:dateTime ;\n",
        "\tcerbo:slug \"index\" .\n",
    );

    fn vault() -> (TempDir, VaultContext) {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".cerbo/objects")).unwrap();
        let ctx = VaultContext {
            vault_path: tmp.path().to_path_buf(),
            global: CerboContext {
                config_dir: tmp.path().join("config"),
                cache_dir: tmp.path().join("cache"),
            },
        };
        (tmp, ctx)
    }

    fn put(ctx: &VaultContext, uuid: &str, file: &str, content: &str) -> PathBuf {
        let dir = ctx.object_path(uuid);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(file);
        fs::write(&path, content).unwrap();
        path
    }

    fn legacy_meta(uuid: &str, title: &str) -> String {
        LEGACY_META.replace(PAGE, uuid).replace("\"Index\"", &format!("\"{title}\""))
    }

    /// An `annotations.ttl` as Cerbo wrote it before the vocabulary change.
    fn legacy_annotations() -> String {
        format!(
            "<cerbo://objects/{PAGE}> <cerbo://ontology/annotation> [ \
             <cerbo://ontology/concept> \"Berlin\" ; <cerbo://ontology/position> \"1,10\" ] .\n"
        )
    }

    fn uuid_n(n: usize) -> String {
        format!("00000000-0000-4000-8000-{n:012}")
    }

    /// Every file under `.cerbo/` with its bytes and modification time.
    fn snapshot(ctx: &VaultContext) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
        walkdir::WalkDir::new(ctx.cerbo_dir())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let path = e.path().to_path_buf();
                let mtime = e.metadata().unwrap().modified().unwrap();
                (path.clone(), (fs::read(&path).unwrap(), mtime))
            })
            .collect()
    }

    #[test]
    fn legacy_meta_is_rewritten_with_the_same_values() {
        let (_tmp, ctx) = vault();
        let path = put(&ctx, PAGE, "meta.ttl", LEGACY_META);
        put(&ctx, PAGE, "page.md", "# Index\n");
        let before = ObjectMeta::read_from_file(&path).unwrap();

        let report = migrate_vocabulary(&ctx, false).unwrap();
        assert_eq!((report.scanned, report.rewritten), (1, 1), "{report:?}");
        assert!(report.errors.is_empty(), "{report:?}");

        let doc = fs::read_to_string(&path).unwrap();
        for legacy in ["cerbo:type", "cerbo:title", "cerbo:Product", "cerbo://ontology/schema/"] {
            assert!(!doc.contains(legacy), "{legacy} survived:\n{doc}");
        }
        let triples = rdf::parse(&doc).unwrap();
        let has = |p: oxrdf::NamedNode, o: oxrdf::Term| {
            triples.iter().any(|t| t.predicate == p && t.object == o)
        };
        assert!(has(rdf::rdf_type(), rdf::schema("NoteDigitalDocument").into()), "{doc}");
        assert!(has(rdf::schema("name"), oxrdf::Literal::new_simple_literal("Index").into()), "{doc}");

        let after = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(after.object_type, before.object_type);
        assert_eq!(after.title, before.title);
        assert_eq!(after.created, before.created);
        assert_eq!(after.modified, before.modified);
        assert_eq!(after.slug, before.slug);
        // Migrating is not editing: page.md and the dates are left alone.
        assert_eq!(fs::read_to_string(ctx.object_path(PAGE).join("page.md")).unwrap(), "# Index\n");
    }

    #[test]
    fn triples_cerbo_does_not_model_are_kept() {
        let (_tmp, ctx) = vault();
        let meta = LEGACY_META.replace(
            "cerbo:slug \"index\" .",
            "cerbo:slug \"index\" ;\n\tcerbo:virtualPath \"notes/index\" ;\n\t\
             <cerbo://ontology/type> <https://schema.org/Recipe> ;\n\t\
             <https://example.org/rating> \"137\" .",
        );
        let path = put(&ctx, PAGE, "meta.ttl", &meta);

        migrate_vocabulary(&ctx, false).unwrap();

        let doc = fs::read_to_string(&path).unwrap();
        let triples = rdf::parse(&doc).unwrap();
        assert!(
            triples.iter().any(|t| t.object == rdf::schema("Recipe").into()),
            "extra class lost:\n{doc}"
        );
        assert!(
            triples.iter().any(|t| t.predicate.as_str() == "https://example.org/rating"),
            "unmodelled triple lost:\n{doc}"
        );
        let meta = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(meta.object_type, ObjectType::Product);
        assert_eq!(meta.virtual_path.as_deref(), Some("notes/index"));
    }

    #[test]
    fn a_fact_in_both_spellings_is_written_once() {
        let (_tmp, ctx) = vault();
        let meta = LEGACY_META.replace(
            "cerbo:slug \"index\" .",
            "cerbo:slug \"index\" ;\n\t<https://schema.org/name> \"Index\" .",
        );
        let path = put(&ctx, PAGE, "meta.ttl", &meta);

        migrate_vocabulary(&ctx, false).unwrap();

        let doc = fs::read_to_string(&path).unwrap();
        assert_eq!(doc.matches("\"Index\"").count(), 1, "{doc}");
    }

    #[test]
    fn a_pre_turtle_meta_is_rewritten_under_its_own_uuid() {
        let (_tmp, ctx) = vault();
        // What the old `format!` writer produced: not Turtle, wrong subject.
        let damaged = concat!(
            "@prefix : <cerbo://ontology/> .\n",
            "<cerbo://objects/placeholder>\n",
            "    :type :Source ;\n",
            "    :title \"RDF :type :Product notes\" ;\n",
            "    :original-url \"https://example.org/137\" ;\n",
            "    schema:dateCreated \"2024-01-01T00:00:00Z\"^^xsd:dateTime .\n",
            "    cerbo:slug \"rdf-notes\" .\n",
        );
        let path = put(&ctx, PAGE, "meta.ttl", damaged);

        let report = migrate_vocabulary(&ctx, false).unwrap();
        assert!(report.errors.is_empty(), "{report:?}");

        let doc = fs::read_to_string(&path).unwrap();
        assert!(doc.contains(&format!("<cerbo://objects/{PAGE}>")), "{doc}");
        let meta = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(meta.object_type, ObjectType::Source);
        assert_eq!(meta.title, "RDF :type :Product notes");
        assert_eq!(meta.original_url.as_deref(), Some("https://example.org/137"));
        assert_eq!(meta.slug.as_deref(), Some("rdf-notes"));
    }

    #[test]
    fn legacy_annotations_are_regenerated_from_the_page() {
        let (_tmp, ctx) = vault();
        // A v1 map: `schema` → the bundled ontology object. The built-in table
        // supplies the namespace.
        fs::write(
            ctx.cerbo_dir().join("ontology-map.json"),
            format!("{{\"prefixes\": {{\"schema\": \"{SCHEMA_ONTOLOGY}\"}}}}"),
        )
        .unwrap();
        put(&ctx, PAGE, "meta.ttl", LEGACY_META);
        put(&ctx, PAGE, "page.md", "# Index\nI visited [Berlin]{schema:Place}.\n");
        let annotations = put(
            &ctx,
            PAGE,
            "annotations.ttl",
            &format!(
                concat!(
                    "@prefix cerbo: <cerbo://ontology/> .\n",
                    "<cerbo://objects/{page}> cerbo:annotation [\n",
                    "    cerbo:concept \"Berlin\" ;\n",
                    "    cerbo:type <cerbo://objects/{ontology}/Place> ;\n",
                    "    cerbo:position \"2,11\" ] .\n",
                ),
                page = PAGE,
                ontology = SCHEMA_ONTOLOGY
            ),
        );

        let report = migrate_vocabulary(&ctx, false).unwrap();
        assert_eq!(report.rewritten, 2, "meta.ttl and annotations.ttl: {report:?}");

        let doc = fs::read_to_string(&annotations).unwrap();
        assert!(!doc.contains("cerbo:annotation") && !doc.contains("cerbo:concept"), "{doc}");
        let triples = rdf::parse(&doc).unwrap();
        let mention = triples
            .iter()
            .find(|t| t.predicate == rdf::schema("mentions"))
            .expect("page mentions something");
        assert_eq!(mention.subject, rdf::object_iri(PAGE).into());
        let node: oxrdf::NamedOrBlankNode = match &mention.object {
            oxrdf::Term::BlankNode(b) => b.clone().into(),
            other => panic!("expected a blank node, got {other}"),
        };
        let of_node = |p: oxrdf::NamedNode| {
            triples
                .iter()
                .find(|t| t.subject == node && t.predicate == p)
                .map(|t| t.object.clone())
        };
        assert_eq!(of_node(rdf::rdf_type()), Some(rdf::schema("Place").into()), "{doc}");
        assert_eq!(
            of_node(rdf::schema("name")),
            Some(oxrdf::Literal::new_simple_literal("Berlin").into())
        );
        assert_eq!(
            of_node(rdf::cerbo("position")),
            Some(oxrdf::Literal::new_simple_literal("2:10").into())
        );
    }

    #[test]
    fn an_annotation_that_lost_its_prefix_is_re_derived_from_the_page() {
        let (_tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", LEGACY_META);
        put(&ctx, PAGE, "page.md", "I visited [Berlin]{schema:Place}.\n");
        let annotations = put(
            &ctx,
            PAGE,
            "annotations.ttl",
            &format!(
                "<cerbo://objects/{PAGE}> <cerbo://ontology/annotation> [ \
                 <cerbo://ontology/concept> \"Berlin\" ; \
                 <cerbo://ontology/type> <cerbo://ontology/Place> ] .\n"
            ),
        );

        migrate_vocabulary(&ctx, false).unwrap();

        let doc = fs::read_to_string(&annotations).unwrap();
        assert!(doc.contains("schema:Place"), "{doc}");
        assert!(!doc.contains("cerbo://ontology/Place") && !doc.contains("cerbo:Place"), "{doc}");
    }

    #[test]
    fn an_unknown_prefix_is_kept_and_warned_about() {
        let (_tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", LEGACY_META);
        put(&ctx, PAGE, "page.md", "[Bob]{nope:Person}\n");
        let annotations = put(&ctx, PAGE, "annotations.ttl", &legacy_annotations());

        let report = migrate_vocabulary(&ctx, false).unwrap();

        assert!(report.warnings.iter().any(|w| w.contains("'nope'")), "{report:?}");
        assert!(fs::read_to_string(&annotations).unwrap().contains("unresolvedType"));
    }

    #[test]
    fn a_page_without_annotations_ttl_does_not_get_one() {
        let (_tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", LEGACY_META);
        put(&ctx, PAGE, "page.md", "[Berlin]{schema:Place}\n");

        migrate_vocabulary(&ctx, false).unwrap();

        assert!(!ctx.object_path(PAGE).join("annotations.ttl").exists());
    }

    #[test]
    fn ontology_objects_keep_their_annotations_file() {
        let (_tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", &LEGACY_META.replace("cerbo:Product", "cerbo:Ontology"));
        put(&ctx, PAGE, "page.md", "[Thing]{schema:Thing}\n");
        let annotations = put(&ctx, PAGE, "annotations.ttl", "# raw\n");

        migrate_vocabulary(&ctx, false).unwrap();

        assert_eq!(fs::read_to_string(&annotations).unwrap(), "# raw\n");
    }

    #[test]
    fn second_run_rewrites_nothing_and_changes_no_mtime() {
        let (_tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", LEGACY_META);
        put(&ctx, PAGE, "page.md", "I visited [Berlin]{schema:Place}.\n");
        put(&ctx, PAGE, "annotations.ttl", &legacy_annotations());
        // An object a current Cerbo wrote is already standard.
        let other = uuid_n(137);
        let standard = ObjectMeta {
            object_type: ObjectType::Source,
            title: "Already standard".into(),
            created: "2024-01-01T00:00:00Z".into(),
            modified: "2024-01-01T00:00:00Z".into(),
            original_url: Some("https://example.org/137".into()),
            mime_type: Some("text/markdown".into()),
            slug: None,
            virtual_path: None,
        };
        put(&ctx, &other, "meta.ttl", &standard.to_turtle(&other));

        let first = migrate_vocabulary(&ctx, false).unwrap();
        assert_eq!(first.rewritten, 2, "only the legacy object: {first:?}");

        let before = snapshot(&ctx);
        let (second, writes) = crate::fsio::audit::recording(|| migrate_vocabulary(&ctx, false).unwrap());
        assert_eq!(second.rewritten, 0, "{second:?}");
        assert!(second.errors.is_empty(), "{second:?}");
        assert!(writes.is_empty(), "second run wrote {writes:?}");
        assert_eq!(snapshot(&ctx), before, "a file or its mtime changed");
    }

    #[test]
    fn dry_run_reports_and_writes_nothing() {
        let (_tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", LEGACY_META);
        put(&ctx, PAGE, "page.md", "[Berlin]{schema:Place}\n");
        put(&ctx, PAGE, "annotations.ttl", &legacy_annotations());

        let before = snapshot(&ctx);
        let (report, writes) = crate::fsio::audit::recording(|| migrate_vocabulary(&ctx, true).unwrap());

        assert_eq!(report.rewritten, 2, "{report:?}");
        assert_eq!(report.files.len(), 2);
        assert!(writes.is_empty(), "dry run wrote {writes:?}");
        assert_eq!(snapshot(&ctx), before);

        // And the real run then does what the dry run promised.
        assert_eq!(migrate_vocabulary(&ctx, false).unwrap().files, report.files);
    }

    #[test]
    fn one_unreadable_object_among_137_is_reported_and_left_alone() {
        let (_tmp, ctx) = vault();
        let broken = uuid_n(42);
        for n in 0..137 {
            let uuid = uuid_n(n);
            put(&ctx, &uuid, "meta.ttl", &legacy_meta(&uuid, &format!("Page {n}")));
        }
        let broken_path = put(&ctx, &broken, "meta.ttl", "this is {{ not turtle at all");

        let report = migrate_vocabulary(&ctx, false).unwrap();

        assert_eq!(report.scanned, 137);
        assert_eq!(report.rewritten, 136, "{:?}", report.errors);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].uuid, broken);
        assert_eq!(fs::read_to_string(&broken_path).unwrap(), "this is {{ not turtle at all");
        for n in (0..137).filter(|&n| n != 42) {
            let doc = fs::read_to_string(ctx.object_path(&uuid_n(n)).join("meta.ttl")).unwrap();
            assert!(doc.contains("schema:NoteDigitalDocument"), "{n}:\n{doc}");
        }
    }

    #[test]
    fn an_ambiguous_object_is_reported_and_left_alone() {
        let (_tmp, ctx) = vault();
        let meta = LEGACY_META.replace(
            "cerbo:type cerbo:Product ;",
            "cerbo:type cerbo:Product , cerbo:Source ;",
        );
        let path = put(&ctx, PAGE, "meta.ttl", &meta);

        let report = migrate_vocabulary(&ctx, false).unwrap();

        assert_eq!(report.errors.len(), 1, "{report:?}");
        assert!(report.errors[0].message.contains("WebPage"), "{report:?}");
        assert_eq!(fs::read_to_string(&path).unwrap(), meta);
    }

    #[test]
    fn a_legacy_source_url_becomes_the_iri_a_new_write_produces() {
        let (_tmp, ctx) = vault();
        let path = put(
            &ctx,
            PAGE,
            "meta.ttl",
            &LEGACY_META
                .replace("cerbo:Product", "cerbo:Source")
                .replace(
                    "cerbo:slug \"index\" .",
                    "cerbo:slug \"index\" ;\n\tcerbo:original-url \"https://example.org/137\" .",
                ),
        );

        migrate_vocabulary(&ctx, false).unwrap();

        let doc = fs::read_to_string(&path).unwrap();
        let triples = rdf::parse(&doc).unwrap();
        let url = triples
            .iter()
            .find(|t| t.predicate == rdf::schema("url"))
            .map(|t| t.object.clone());
        assert_eq!(
            url,
            Some(oxrdf::NamedNode::new_unchecked("https://example.org/137").into()),
            "schema:url must be an IRI:\n{doc}"
        );
        // Exactly what a save of the same object writes.
        let meta = ObjectMeta::read_from_file(&path).unwrap();
        assert_eq!(meta.object_type, ObjectType::Source);
        assert_eq!(doc, meta.to_turtle(PAGE));
    }

    #[test]
    fn a_pre_turtle_meta_with_two_classes_is_reported_and_left_alone() {
        let (_tmp, ctx) = vault();
        let damaged = concat!(
            "@prefix : <cerbo://ontology/> .\n",
            "<cerbo://objects/placeholder>\n",
            "    :type :Source ;\n",
            "    :type :Product ;\n",
            "    :title \"Both\" .\n",
            "    cerbo:slug \"both\" .\n",
        );
        let path = put(&ctx, PAGE, "meta.ttl", damaged);

        let report = migrate_vocabulary(&ctx, false).unwrap();

        assert_eq!(report.errors.len(), 1, "{report:?}");
        assert_eq!(report.errors[0].uuid, PAGE);
        assert!(report.errors[0].message.contains("WebPage"), "{report:?}");
        assert_eq!(fs::read_to_string(&path).unwrap(), damaged);
    }

    /// A vault written entirely by the current Cerbo, with a page edited since
    /// its last index run: nothing in it is legacy, so nothing is rewritten.
    #[test]
    fn a_standard_vault_with_stale_annotations_is_left_alone() {
        let (_tmp, ctx) = vault();
        let meta = ObjectMeta::read_from_file(&put(&ctx, PAGE, "meta.ttl", LEGACY_META)).unwrap();
        put(&ctx, PAGE, "meta.ttl", &meta.to_turtle(PAGE));
        put(&ctx, PAGE, "page.md", "[Berlin]{schema:Place}\n");
        let map = PrefixMap::load_vault(&ctx).unwrap();
        let (indexed, _) = annotations_ttl(&map, PAGE, &extract_annotations("[Berlin]{schema:Place}\n"));
        put(&ctx, PAGE, "annotations.ttl", &indexed);
        // Edited after indexing.
        put(&ctx, PAGE, "page.md", "[Berlin]{schema:Place} and [Alice]{foaf:Person}\n");

        let before = snapshot(&ctx);
        let (report, writes) = crate::fsio::audit::recording(|| migrate_vocabulary(&ctx, false).unwrap());

        assert_eq!(report.rewritten, 0, "{report:?}");
        assert!(report.errors.is_empty(), "{report:?}");
        assert!(writes.is_empty(), "wrote {writes:?}");
        assert_eq!(snapshot(&ctx), before);
    }

    #[test]
    fn trashed_objects_are_skipped() {
        let (_tmp, ctx) = vault();
        let trashed = ctx.cerbo_dir().join("trash").join(format!("20260101T000000Z-{PAGE}"));
        fs::create_dir_all(&trashed).unwrap();
        fs::write(trashed.join("meta.ttl"), LEGACY_META).unwrap();

        let report = migrate_vocabulary(&ctx, false).unwrap();

        assert_eq!((report.scanned, report.rewritten), (0, 0));
        assert_eq!(fs::read_to_string(trashed.join("meta.ttl")).unwrap(), LEGACY_META);
    }
}
