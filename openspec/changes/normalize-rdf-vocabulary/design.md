# Design

## Context

- All Turtle is produced and parsed in `core/src/rdf.rs` (`oxrdf` + `oxttl`). Namespace constants: `NS_CERBO = cerbo://ontology/`, `NS_SCHEMA = cerbo://ontology/schema/`.
- `meta.ttl` uses `cerbo:title`, `cerbo:original-url` and `cerbo:mime-type` as literals (`core/src/object.rs`, `ObjectMeta`).
- `meta.ttl` writes `cerbo:type cerbo:<Type>`.
- `core/src/annotations.rs::resolve_prefix_to_uri`:
  - reads `ontology-map.json` from the **global** config dir, while `cerbo init` writes it into the vault's `.cerbo/`;
  - maps a known prefix to `cerbo://objects/<uuid>/<Type>`;
  - maps any other prefix to `cerbo://ontology/<Type>`, so the prefix is lost.
- Annotations are `cerbo:annotation` nodes with `cerbo:concept` and are typed with `cerbo:type`; positions are written as `line,column` (the spec says `line:column`), and `annotations.ttl` is not rewritten when the last annotation is removed.
- `cerbo import-ontology` never touches the prefix map.

See proposal.md for motivation. This change must land before `add-oxigraph-semantic-index`.

## Goals / Non-Goals

**Goals:**
- One table of namespaces and legacy aliases, used by every reader and writer.
- Old vaults readable without migration; migration is explicit and idempotent.

**Non-Goals:**
- Migrating automatically on startup, or from the desktop app.
- Keeping the legacy forms writable.

## Decisions

### D1. schema.org classes replace Cerbo's classes (user decision)

| Before | After | Behaviour |
|---|---|---|
| `cerbo:Product` | `schema:NoteDigitalDocument` | editable note |
| `cerbo:Source` | `schema:WebPage` | read-only |
| `cerbo:Attachment` | `schema:MediaObject` | binary file |
| `cerbo:Ontology` | `schema:DefinedTermSet` | ontology documentation |

`ObjectType` is decided by which of these four classes `meta.ttl` carries. Other `rdf:type` values are allowed and ignored (a user may add `schema:Recipe` to a note). Two or more of the four is an error naming the UUID; the object is neither read nor written, because guessing could make a note read-only or vice versa.

- **Alternatives:**
  - Dual typing (Cerbo role + schema.org class): safer against hand edits, but keeps `cerbo:*` classes. The user chose schema.org only.
  - `owl:Ontology` for ontologies: more precise, but not schema.org. `DefinedTermSet` ("a set of defined terms") is close enough, and `rdf:type owl:Ontology` can be added later as an extra class.

### D2. Property mapping and legacy aliases in the parser

| Legacy (read only) | Standard (read + write) |
|---|---|
| `cerbo://ontology/type` | `rdf:type` |
| `cerbo:Product` / `Source` / `Attachment` / `Ontology` (objects) | schema.org classes from D1 |
| `cerbo://ontology/title` | `schema:name` |
| `cerbo://ontology/original-url` (literal) | `schema:url` (IRI; literal also read) |
| `cerbo://ontology/mime-type` | `schema:encodingFormat` |
| `cerbo://ontology/schema/<x>` | `https://schema.org/<x>` |
| `cerbo:annotation` / `cerbo:concept` | `schema:mentions` / `schema:name` |

`rdf::parse` rewrites legacy predicates as it reads, so `ObjectMeta`, annotation readers and, later, the semantic index see only standard terms. Unchanged: `cerbo:slug`, `cerbo:virtualPath`, `cerbo:position`, and the new `cerbo:unresolvedType`, which have no schema.org equivalent.

- **Alternatives:**
  - Teaching each reader both spellings spreads the legacy forms through the code.
  - A one-shot migration with no tolerant reading breaks every vault not yet migrated, including vaults synced from a machine running an older Cerbo.
  - `dcterms:title` instead of `schema:name`: equally standard, but mixing two vocabularies for the same object adds no value; schema.org already supplies the dates.

### D2a. Annotations as mentioned entities

`[Berlin]{schema:Place}` becomes:

```turtle
<cerbo://objects/<uuid>> schema:mentions [
    a schema:Place ; schema:name "Berlin" ; cerbo:position "1:10" ] .
```

`schema:mentions` has domain `schema:CreativeWork`, which `NoteDigitalDocument` is. A consumer that knows only schema.org reads it as "this note mentions a place named Berlin". The node stays blank and per page. Merging equal mentions into shared entities is a follow-up; the shape is already the one that change will need.

### D3. Prefix map v2 in the vault

The file stays `.cerbo/ontology-map.json`. New shape:

```json
{ "version": 2,
  "prefixes": { "ex": { "namespace": "https://example.org/vocab#", "ontology": "<uuid>" } } }
```

- A string value (v1: prefix → UUID) is still read. It resolves only if the prefix is well known.
- The resolver takes the `VaultContext` and stops using the global config dir.
- Built-in prefixes are a constant table. Vault entries win.

### D4. Unresolvable prefix → `cerbo:unresolvedType` literal

Inventing an IRI hides the problem and produces data that never matches anything. Dropping the annotation loses what the user wrote. A literal keeps it queryable (`?a cerbo:unresolvedType ?t`) and lets a later reindex upgrade it once the prefix is registered.

### D5. Migration regenerates annotations from `page.md`

- `cerbo migrate vocabulary` parses `meta.ttl` with the tolerant parser and reserialises it, which also adds the schema.org class.
- `annotations.ttl` is regenerated from `page.md` with the new resolver rather than rewritten IRI by IRI. This also repairs types that lost their prefix (`cerbo://ontology/Place`), the `line,column` position format and stale files.
- Each new document is compared byte-for-byte with the file on disk and written (via `fsio` atomic replace) only if it differs. This is what makes a second run touch nothing.

### D6. `import-ontology --prefix --namespace`

Detecting the namespace from a fetched page is unreliable: the current import fetches HTML documentation, not RDF. Explicit flags are predictable. Both are required together; without them the import behaves as today and warns.

### D7. Prefixes in serialised files

The serializer declares `rdf:`, `schema:` (`https://schema.org/`), `cerbo:` and `xsd:`, so the common case reads as `a schema:NoteDigitalDocument`. Annotation types from other namespaces are written as full IRIs. A per-file prefix list derived from the map is not worth the churn.

## Risks / Trade-offs

- [Mixed-version sync: an older Cerbo on another device reads a migrated `meta.ttl` and does not find `cerbo:type`] → Note it in the release notes and the man page: upgrade every device before migrating. Old builds are not changed.
- [Tolerant parsing keeps legacy aliases forever] → The alias table is tiny. A later change can drop it after a deprecation period.
- [Regenerating annotations depends on `page.md` being parseable] → Errors are reported per UUID and the old `annotations.ttl` is left untouched (spec: unreadable object).
- [`schema:dateModified` is not a property of every class in schema.org] → RDF does not enforce domains; this is acceptable.
- [A hand edit adds a second of the four classes] → Explicit error naming the UUID and both classes; `cerbo migrate vocabulary` reports it too.
- [Tools that read `schema:name` as a display name may show page titles differently from `cerbo:title` consumers] → No external consumer of `cerbo:title` exists today; the desktop and CLI read through `ObjectMeta`.

## Migration Plan

1. Ship the tolerant reader and the new writer. Any saved object is upgraded naturally.
2. Users run `cerbo migrate vocabulary --dry-run`, then `cerbo migrate vocabulary`, once per machine set (the files sync).
3. `test-vault` is migrated in the same commit as the implementation.
4. Rollback: an older build cannot read migrated files. Restore from git or sync history. The `--dry-run` step and the release note warn about this.
