# Cerbo Storage Layout (UUID-Based Model)

## Overview

Cerbo stores every object under a UUID. A vault keeps all of it in a `.cerbo/`
directory at the vault root. Alongside it, `cerbo symlink` materialises a
human-navigable `cerbo/` tree of symlinks into that store — the store is the
truth, the tree is a projection you can throw away and rebuild.

## Directory Structure

```
my-vault/
├── .cerbo/                      # Vault metadata directory — the source of truth
│   ├── objects/                 # UUID-based object storage
│   │   ├── <uuid-1>/            # Object directory
│   │   │   ├── page.md          # Page content (Product, Source, Ontology)
│   │   │   ├── meta.ttl         # Object metadata (Turtle RDF)
│   │   │   ├── annotations.ttl  # HackMD annotations
│   │   │   └── <file>           # Attached file (Attachment type)
│   │   ├── <uuid-2>/
│   │   └── ...
│   ├── trash/                   # Deleted objects, kept until you remove them
│   │   └── <timestamp>-<uuid>/  # One entry per deletion, contents intact
│   ├── ontology-map.json        # Annotation prefix → namespace IRI (+ ontology UUID)
│   └── ...
├── cerbo/                       # Symlink projection, rebuilt by `cerbo symlink`
│   ├── rust-ownership.md -> ../.cerbo/objects/<uuid>/page.md
│   └── notes/
│       └── rust.md -> ../../.cerbo/objects/<uuid>/page.md
├── .gitignore                   # Ignores /cerbo/ and /.cerbo/trash/
├── actual-content.md            # Regular files in the vault (not managed by Cerbo)
└── other-files/
```

## Object Types

Each object's class is recorded in `meta.ttl` as `rdf:type` with a schema.org
class. The class decides how Cerbo treats the object:

| Type | `rdf:type` | Behaviour |
|------|------------|-----------|
| Product | `schema:NoteDigitalDocument` | editable note |
| Source | `schema:WebPage` | read-only |
| Attachment | `schema:MediaObject` | binary file |
| Ontology | `schema:DefinedTermSet` | ontology documentation |

An object may carry further `rdf:type` values (for example `schema:Recipe` on a
note); Cerbo ignores them. An object that carries two or more of the four classes
above is refused with an error naming its UUID and both classes: Cerbo neither
reads nor rewrites it until one class is removed by hand.

### 1. Product (Page)
- **Directory**: `.cerbo/objects/<uuid>/`
- **Files**: `page.md`, `meta.ttl`, `annotations.ttl`
- **Read-only**: No
- **Description**: Regular wiki pages created by users

### 2. Source
- **Directory**: `.cerbo/objects/<uuid>/`
- **Files**: `page.md`, `meta.ttl`
- **Read-only**: Yes (cannot write or delete)
- **Description**: Imported from URL via `cerbo import <url>`

### 3. Attachment
- **Directory**: `.cerbo/objects/<uuid>/`
- **Files**: `<filename>`, `meta.ttl` (no `page.md`)
- **Read-only**: No
- **Description**: File attachments (images, PDFs, etc.)

### 4. Ontology
- **Directory**: `.cerbo/objects/<uuid>/`
- **Files**: `page.md`, `meta.ttl`
- **Read-only**: No
- **Description**: Imported ontologies (Schema.org, FOAF, etc.)

## How writes reach disk

Every file Cerbo writes into a vault is replaced atomically: the bytes go to a
temporary file in the destination's own directory, are flushed to storage, and
are then renamed over the destination, after which the directory entry itself is
flushed. A reader always sees either the complete previous content or the
complete new content, and a write that fails part-way — a full disk, a killed
process — leaves the previous content intact.

A crash between the write and the rename can leave a `.cerbo-tmp-*` sibling
behind. Those are ignored by page listings, link resolution, indexing and the
symlink projection, and are safe to delete.

## The symlink projection (`cerbo/`)

`cerbo symlink` rebuilds `cerbo/` from scratch out of the object store. Every
Product and Source object with a `page.md` becomes one relative symlink named
`<slug>.md`, nested under its `cerbo:virtualPath` if it has one. Relative targets
keep the repository portable.

- The tree is derived, never authoritative. Deleting it loses nothing.
- Two objects that would land on the same path are a **collision**: the rebuild
  refuses and names both UUIDs. Resolve it by changing one object's
  `cerbo:slug` or `cerbo:virtualPath`.
- The rebuild refuses to wipe a `cerbo/` that contains anything other than
  symlinks into `.cerbo/objects/`, so a stray file of yours is never destroyed.
- Attachments, Ontology objects, trashed objects and objects without a `page.md`
  produce no symlink.
- `cerbo/` is in `.gitignore`; commit `.cerbo/` instead.

## Trash

`cerbo page delete` moves the object's directory to
`.cerbo/trash/<timestamp>-<uuid>/` instead of destroying it. Every file inside is
preserved byte-for-byte — the move is a rename, not a copy — so recovery is a
matter of moving the directory back under `.cerbo/objects/<uuid>/`.

- The timestamp makes each deletion its own entry: deleting, recreating and
  deleting the same UUID again leaves two entries, neither overwriting the other.
- Read-only (Source) objects still cannot be deleted, and the refusal happens
  before anything moves.
- Trash is outside the live object set. Indexing, the semantic index, page
  listing, link resolution and the symlink projection all ignore it. A live page
  that still links to a trashed object reports a **broken link**; the object is
  never resurrected.
- **Nothing prunes the trash.** It grows until you remove entries yourself.
  `cerbo init` adds `/.cerbo/trash/` to `.gitignore`.

## File Formats

`meta.ttl` and `annotations.ttl` are valid Turtle, written and
read with a real RDF parser. Every prefix used is declared in the document, and
every literal is escaped, so any character a title can hold round-trips exactly.

### meta.ttl (Turtle RDF)
```turtle
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix schema: <https://schema.org/> .
@prefix cerbo: <cerbo://ontology/> .
<cerbo://objects/f7e435db-9740-4a2a-8e57-b439c4f8bb18> a schema:NoteDigitalDocument ;
	schema:name "My Page" ;
	schema:dateCreated "2024-01-01T00:00:00Z"^^xsd:dateTime ;
	schema:dateModified "2024-01-01T00:00:00Z"^^xsd:dateTime ;
	cerbo:slug "my-page" ;
	cerbo:virtualPath "notes/rust" .
```

The subject is the object's own IRI. `schema:url` appears only on Source objects
and holds the imported address as an IRI (`schema:url <https://example.com/>`);
`schema:encodingFormat` (the MIME type) appears only on Attachments.
`schema:name` is the **only** home of a page's title: the H1 in `page.md` is a
convenience copy, and Cerbo never reads it back.

`schema:` is the real schema.org namespace, `https://schema.org/`. Terms that
have no schema.org equivalent stay in Cerbo's own namespace: `cerbo:slug`,
`cerbo:virtualPath`, `cerbo:position` and `cerbo:unresolvedType`.

#### Legacy vocabulary

Vaults written by older Cerbo builds use Cerbo's private terms. They are still
read: the parser rewrites them to the standard terms as it reads, and every write
uses only the standard ones.

| Legacy (read only) | Standard (read and write) |
|---|---|
| `cerbo:type` | `rdf:type` (`a`) |
| `cerbo:Product` / `Source` / `Attachment` / `Ontology` | `schema:NoteDigitalDocument` / `WebPage` / `MediaObject` / `DefinedTermSet` |
| `cerbo:title` | `schema:name` |
| `cerbo:original-url` (literal) | `schema:url` (IRI; a literal is still read) |
| `cerbo:mime-type` | `schema:encodingFormat` |
| `<cerbo://ontology/schema/x>` | `<https://schema.org/x>` |
| `cerbo:annotation` / `cerbo:concept` | `schema:mentions` / `schema:name` |

`cerbo migrate vocabulary` rewrites a vault's files into the standard form once
(see [Migrating an old vault](#migrating-an-old-vault)). Saving an object also
rewrites its `meta.ttl` in the standard form.

### annotations.ttl (Turtle RDF)

Each annotation becomes an entity the page mentions. For a `page.md` whose third
line is `I visited [Berlin]{schema:Place} with [Bob]{ex:Person}.` and a vault
with no `ex` prefix registered:

```turtle
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix schema: <https://schema.org/> .
@prefix cerbo: <cerbo://ontology/> .
<cerbo://objects/<uuid>> schema:mentions _:m0 .
_:m0 schema:name "Berlin" ;
	a schema:Place ;
	cerbo:position "3:10" .
<cerbo://objects/<uuid>> schema:mentions _:m1 .
_:m1 schema:name "Bob" ;
	cerbo:unresolvedType "ex:Person" ;
	cerbo:position "3:38" .
```

- `rdf:type` is the full type IRI: the prefix's namespace followed by the local
  name. Types outside `schema:` are written as full IRIs, for example
  `a <http://xmlns.com/foaf/0.1/Person>`.
- `cerbo:position` is `"line:column"`: the 1-based line and the 0-based character
  offset of the opening `[` on that line.
- An annotation whose prefix cannot be resolved (unknown, or missing as in
  `[Bob]{Person}`) is kept with `cerbo:unresolvedType "prefix:Type"` and no
  `rdf:type`, and indexing prints a warning naming the prefix. Once the prefix is
  registered, `cerbo index` turns it into a real type.
- The file is rewritten on every index of the page. A page whose last annotation
  was removed gets a file with no `schema:mentions` statements.

### ontology-map.json

The vault's own `.cerbo/ontology-map.json` maps each annotation prefix to a
namespace IRI and, optionally, to the ontology object that documents it:

```json
{
  "version": 2,
  "prefixes": {
    "ex": {
      "namespace": "https://example.org/vocab#",
      "ontology": "123e4567-e89b-12d3-a456-426614174000"
    }
  }
}
```

- Built-in prefixes need no entry: `schema` (`https://schema.org/`), `foaf`,
  `dcterms`, `skos`, `rdf`, `rdfs`, `owl` and `xsd`, each with its usual
  namespace. A vault entry for one of them wins over the built-in one.
- Entries are added by `cerbo import-ontology <url> --prefix ex --namespace
  https://example.org/vocab#`. `cerbo init` writes an empty map
  (`{"version": 2, "prefixes": {}}`), plus entries for the bundled Schema.org and
  FOAF ontologies when it could fetch them.
- A version 1 map (`"prefix": "<uuid>"`, no namespace) is still read. Such an
  entry resolves only for a built-in prefix; register any other prefix again with
  `--prefix` and `--namespace`.
- Only the vault's own map is used. Two vaults may bind the same prefix to
  different namespaces.

## The semantic index (outside the vault)

Backlinks and SPARQL queries are answered by a derived index of each vault's
knowledge graph. It lives in the user cache, never in the vault:

```
$XDG_CACHE_HOME/cerbo/index/<key>/   # ~/.cache/cerbo/index/ by default
  LOCK                      # held by one Cerbo process for one operation
  state.json                # live generation, format version, vault path
  graph-<gen>/              # Oxigraph store, one named graph per object
  fingerprints-<gen>.bin    # file stats used to notice outside changes
```

`<key>` is the first 16 hex characters of the SHA-256 of the vault's canonical
path, so every vault on a machine has its own index.

- The graph holds each object's `meta.ttl` and `annotations.ttl` triples plus
  one `<page> schema:citation <target>` per distinct `cerbo://` link in its
  `page.md`, all in the named graph `cerbo://graphs/<uuid>`. Backlinks of X are
  the pages citing X.
- It is built by the first command that needs it, rebuilt automatically when it
  comes from another format version or another vault path, and caught up before
  every answer with files changed by git, sync or an editor. Saving a page
  updates only that page's graph.
- `cerbo index` rebuilds it into a new generation and switches to it atomically;
  an interrupted rebuild leaves the previous one working. `cerbo index --page`
  replaces only that page's graph.
- Deleting the directory loses nothing. Indexes of moved or deleted vaults stay
  in the cache until removed by hand.
- `cerbo sparql '<query>'` queries it read-only; see the man page.

### Legacy `backrefs.ttl`

Older Cerbo builds cached backlinks in a `backrefs.ttl` file inside each linked
object (`<target> cerbo:hasBacklink <source>`), so saving one page rewrote other
objects' files. Cerbo neither writes nor reads these files any more. A leftover
file is ignored, and the next full `cerbo index` deletes every one in live
objects and reports the count (`legacy_backrefs_removed` with `--json`).

**Upgrade Cerbo on every device that syncs the vault.** An older build keeps
writing `backrefs.ttl` (ignored here and removed again by the next full
`cerbo index`) and reads backlinks only from those files. To roll back, restore
the files from git or sync history, or run the older build's `cerbo index`.

## Link Format

Cerbo uses `cerbo://<uuid>` links instead of wikilinks:

```markdown
Check out [this page](cerbo://123e4567-e89b-12d3-a456-426614174000) for details.

[Bob]{schema:Person} works at [Acme Corp]{schema:Organization}.
```

Title-to-UUID resolution is a scan of the vault's `meta.ttl` files. There is no
lookup index on disk to fall out of date.

## Annotation Syntax (HackMD-style)

```
[Text]{prefix:Type}
```

Examples:
- `[Bob]{schema:Person}` → the page mentions a `https://schema.org/Person` named "Bob"
- `[Alice]{foaf:Person}` → the page mentions a `http://xmlns.com/foaf/0.1/Person` named "Alice"
- `[Product X]{Product}` → no prefix: kept as `cerbo:unresolvedType "Product"`, with a warning

## Migrating an old vault

Old files stay readable, so migration is optional, but it makes the files on
disk match what standard RDF tools expect:

```bash
cerbo migrate vocabulary --dry-run      # list what would change
cerbo migrate vocabulary                # rewrite every registered vault
cerbo migrate vocabulary --vault ~/notes  # or just one vault
```

The command rewrites a `meta.ttl` only when it uses a legacy term (writing it
exactly as a save would), regenerates an existing `annotations.ttl` from
`page.md` only when it uses a legacy term, writes each file atomically and
only when its bytes change, and skips the trash. A second run changes nothing.
An object it cannot read is named by UUID and left as it was.

**Upgrade Cerbo on every device that syncs the vault before migrating.** Older
builds cannot read the migrated files.

## Key Differences from Slug-Based Model

| Feature | Old Model (Slug-Based) | New Model (UUID-Based) |
|---------|------------------------|------------------------|
| Storage | `<slug>/page.md` | `.cerbo/objects/<uuid>/page.md` |
| Human-readable tree | The storage itself | `cerbo/` symlink projection |
| Links | `[[Page Title]]` | `cerbo://<uuid>` |
| Backlinks | Computed on-the-fly | Query over the semantic index in the user cache |
| Annotations | Not supported | HackMD syntax → `annotations.ttl` |
| Renaming | Updates all links | Updates `meta.ttl` only; links are UUIDs |
| Delete | Removed the files | Moved to `.cerbo/trash/` |
| Read-only | Not enforced | Enforced for Source type |

## Breaking Change

This is a **breaking change** by design. Old vaults using slug-based storage are
not compatible. Users must create new vaults and manually migrate content if
needed.

A `.cerbo/index.json` from an older Cerbo is ignored: nothing reads it, and it may
be deleted by hand at any time.
