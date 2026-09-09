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
│   │   │   ├── backrefs.ttl     # Incoming links (backlinks)
│   │   │   ├── annotations.ttl  # HackMD annotations
│   │   │   └── <file>           # Attached file (Attachment type)
│   │   ├── <uuid-2>/
│   │   └── ...
│   ├── trash/                   # Deleted objects, kept until you remove them
│   │   └── <timestamp>-<uuid>/  # One entry per deletion, contents intact
│   ├── ontology-map.json        # Prefix → UUID mapping for ontologies
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

### 1. Product (Page)
- **Directory**: `.cerbo/objects/<uuid>/`
- **Files**: `page.md`, `meta.ttl`, `backrefs.ttl`, `annotations.ttl`
- **Read-only**: No
- **Description**: Regular wiki pages created by users

### 2. Source
- **Directory**: `.cerbo/objects/<uuid>/`
- **Files**: `page.md`, `meta.ttl`, `backrefs.ttl`
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
- Trash is outside the live object set. Indexing, page listing, link resolution,
  backreference maintenance and the symlink projection all ignore it. A live page
  that still links to a trashed object reports a **broken link**; the object is
  never resurrected.
- **Nothing prunes the trash.** It grows until you remove entries yourself.
  `cerbo init` adds `/.cerbo/trash/` to `.gitignore`.

## File Formats

`meta.ttl`, `backrefs.ttl` and `annotations.ttl` are valid Turtle, written and
read with a real RDF parser. Every prefix used is declared in the document, and
every literal is escaped, so any character a title can hold round-trips exactly.

### meta.ttl (Turtle RDF)
```turtle
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix schema: <cerbo://ontology/schema/> .
@prefix cerbo: <cerbo://ontology/> .
<cerbo://objects/f7e435db-9740-4a2a-8e57-b439c4f8bb18> cerbo:type cerbo:Product ;
	cerbo:title "My Page" ;
	schema:dateCreated "2024-01-01T00:00:00Z"^^xsd:dateTime ;
	schema:dateModified "2024-01-01T00:00:00Z"^^xsd:dateTime ;
	cerbo:slug "my-page" ;
	cerbo:virtualPath "notes/rust" .
```

The subject is the object's own IRI. `cerbo:original-url` appears only on Source
objects and `cerbo:mime-type` only on Attachments. `cerbo:title` is the **only**
home of a page's title: the H1 in `page.md` is a convenience copy, and Cerbo never
reads it back.

### backrefs.ttl (Turtle RDF)
```turtle
@prefix cerbo: <cerbo://ontology/> .
<cerbo://objects/<target-uuid>> cerbo:hasBacklink <cerbo://objects/<source-uuid>> ,
	<cerbo://objects/<another-source-uuid>> .
```

Derived data: `cerbo index` rebuilds it from page content in a single pass, one
write per target. An object with no inbound links has a `backrefs.ttl` holding no
`cerbo:hasBacklink` statements. Cerbo never writes a `backrefs.ttl` for an object
that does not exist.

### annotations.ttl (Turtle RDF)
```turtle
@prefix cerbo: <cerbo://ontology/> .
<cerbo://objects/<uuid>> cerbo:annotation _:a0 , _:a1 .
_:a0 cerbo:concept "Bob" ;
	cerbo:type <cerbo://objects/<uuid-schema>/Person> ;
	cerbo:position "5,10" .
_:a1 cerbo:concept "Alice" ;
	cerbo:type <cerbo://objects/<uuid-foaf>/Person> ;
	cerbo:position "15,20" .
```

### ontology-map.json
```json
{
  "prefixes": {
    "schema": "123e4567-e89b-12d3-a456-426614174000",
    "foaf": "987fcdeb-51a2-43d1-b234-567890123456"
  }
}
```

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
- `[Bob]{schema:Person}` → Annotation with concept "Bob", type Person from schema ontology
- `[Alice]{foaf:Person}` → Annotation with concept "Alice", type Person from FOAF ontology
- `[Product X]{Product}` → Annotation with no prefix (uses default ontology)

## Key Differences from Slug-Based Model

| Feature | Old Model (Slug-Based) | New Model (UUID-Based) |
|---------|------------------------|------------------------|
| Storage | `<slug>/page.md` | `.cerbo/objects/<uuid>/page.md` |
| Human-readable tree | The storage itself | `cerbo/` symlink projection |
| Links | `[[Page Title]]` | `cerbo://<uuid>` |
| Backlinks | Computed on-the-fly | Cached in `backrefs.ttl` |
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
