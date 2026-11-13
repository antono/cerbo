# Proposal

## Why

Cerbo writes RDF, but almost every term is Cerbo's own, even where schema.org has a standard one:

- Types use `cerbo:type`, not `rdf:type`, so standard tools and SPARQL `a` do not see them.
- `schema:` points to `cerbo://ontology/schema/` instead of `https://schema.org/`.
- Titles, source URLs and MIME types use `cerbo:` predicates that have schema.org equivalents.
- `[Berlin]{schema:Place}` becomes `cerbo://objects/<uuid>/Place` (or `cerbo://ontology/Place` when the prefix is unknown), so it never matches `https://schema.org/Place`.

Search engines, LLMs and RDF tools understand schema.org without explanation. Fixing this now is cheap, and expensive once `add-oxigraph-semantic-index` ships queries people depend on. This change lands first.

## What Changes

**BREAKING (on-disk)** for `meta.ttl` and `annotations.ttl`. Old files stay readable.

- Object classes become schema.org classes, recorded with `rdf:type`. They replace Cerbo's own classes and decide behaviour:
  - `cerbo:Product` → `schema:NoteDigitalDocument`
  - `cerbo:Source` → `schema:WebPage` (read-only)
  - `cerbo:Attachment` → `schema:MediaObject`
  - `cerbo:Ontology` → `schema:DefinedTermSet`
- `schema:` means `https://schema.org/`:
  - `cerbo:title` → `schema:name`
  - `cerbo:original-url` → `schema:url`
  - `cerbo:mime-type` → `schema:encodingFormat`
  - `dateCreated` and `dateModified` become real schema.org terms.
- An annotation becomes an entity the page mentions: `<page> schema:mentions [ a <https://schema.org/Place> ; schema:name "Berlin" ; cerbo:position "1:10" ]`.
- An unresolvable prefix keeps the annotation with `cerbo:unresolvedType` and a warning.
- Terms with no good schema.org equivalent stay in `cerbo:`: `slug`, `virtualPath`, `position`, `unresolvedType`, and the index's `similarTo`. Page-to-page links in the index use `schema:citation`.
- Annotations read the vault's own `.cerbo/ontology-map.json` (today a global copy). It gives each prefix a namespace IRI. Well-known prefixes (`schema`, `foaf`, `dcterms`, `skos`, `rdf`, `rdfs`, `owl`, `xsd`) need no entry.
- `cerbo import-ontology` gains `--prefix` and `--namespace`.
- Reads accept legacy forms. Writes use only the new ones.
- New `cerbo migrate vocabulary` (`--vault`, `--dry-run`, `--json`) rewrites existing metadata. It is idempotent.

## Capabilities

### New Capabilities

- `vocabulary-migration`: `cerbo migrate vocabulary`.

### Modified Capabilities

- `uuid-object-storage`: schema.org classes and properties in `meta.ttl`; annotations as `schema:mentions`.
- `semantic-annotations`: mentioned entities with full type IRIs; unresolvable prefixes reported.
- `ontology-management`: per-vault prefix map with namespace IRIs, built-in prefixes, explicit registration.

## Non-goals

- Changing the `cerbo://` scheme of object IRIs.
- Merging equal mentions across pages into one shared entity. This change prepares for it.
- Timestamp precision.
- Checking that a type exists in its ontology.

## Impact

- **Code:** `core/src/rdf.rs` (namespaces, alias table), `object.rs` (`ObjectMeta`), `annotations.rs`, `cli/src/main.rs` (`init`, `import-ontology`, `migrate`).
- **Data:** run `cerbo migrate vocabulary` once per vault.
- **Desktop:** no UI change.
- **Docs:** `cli/man/cerbo.md`, `docs/storage.md`, `README.md`.
- **Follow-on:** `add-oxigraph-semantic-index` queries `schema:name`, `schema:mentions` and `schema:citation`.
