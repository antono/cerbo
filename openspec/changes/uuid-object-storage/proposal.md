## Why

Cerbo currently uses human-readable slugs derived from page titles as the primary identifier (`rust-ownership/page.md`). This couples identity to presentation and limits the system's semantic capabilities. A UUID-based object storage model decouples identity from display, enables machine-readable semantic annotations via [HackMD Semantic Markdown](https://hackmd.io/@sparna/semantic-markdown-draft), and lays the foundation for a proper RDF triple store ([Oxigraph](https://oxigraph.org/)) with SPARQL querying.

References:
- [Local-first software](https://www.inkandswitch.com/essay/local-first/)
- [Oxigraph - Rust RDF store](https://oxigraph.org/)
- [Turtle RDF Syntax (W3C)](https://www.w3.org/TR/turtle/)
- [Schema.org schemas](https://schema.org/docs/schemas.html)
- [FOAF Vocabulary Specification](http://xmlns.com/foaf/spec/)
- [HackMD Semantic Markdown Draft](https://hackmd.io/@sparna/semantic-markdown-draft)

## What Changes

- **BREAKING**: Replace slug-based page storage (`<slug>/page.md`) with UUID-based object storage (`.cerbo/objects/<uuid>/page.md`)
- **BREAKING**: Replace `[[Page Title]]` wikilinks with `cerbo://<uuid>` markdown links
- New `cerbo init` command to initialize a vault (creates `.cerbo/` directory with bundled ontologies)
- New file layout per object:
  - `page.md` — content (for Page/Source types), contains `cerbo://<uuid>` links
  - `meta.ttl` — RDF metadata (type, title, dates, mime-type, original-url)
  - `backrefs.ttl` — cached backlinks (incoming `cerbo://` links from OTHER objects)
  - `annotations.ttl` — HackMD-style `[Text]{prefix:Type}` semantic annotations
- New `cerbo import <url>` command → creates `type: source` (read-only after creation)
- New `cerbo create "Title"` command → creates `type: product` (editable)
- New `cerbo import-ontology <url>` command → creates `type: ontology` objects
- Bundled ontologies (Schema.org, FOAF) created on `cerbo init`
- New `cerbo resolve <uuid>` command → returns local path to object
- Index strategy: `.cerbo/index.json` (title→uuid, uuid→path) now; Oxigraph embedded triple store later
- Link format simplified: `cerbo://<uuid>` (no `/page.md` or `/image.png` suffix)
- HackMD-style semantic annotations for non-link metadata: `[Berlin]{schema:Place}`

### Storage Layout

```
.cerbo/
├── ontology-map.json          ← prefix → uuid mapping
│                           e.g., { "schema": "<uuid>", "foaf": "<uuid>" }
└── objects/
    ├── <uuid-schema-org>/      ← bundled ontology (type: ontology)
    │   ├── page.md
    │   └── meta.ttl
    │
    ├── <uuid-foaf>/             ← bundled ontology (type: ontology)
    │   ├── page.md
    │   └── meta.ttl
    │
    ├── <uuid-page-1>/          ← user page (type: product)
    │   ├── page.md              ← contains [Text](cerbo://<uuid>) links
    │   ├── meta.ttl             ← type, title, created, modified
    │   ├── relations.ttl        ← outgoing cerbo:// links
    │   └── annotations.ttl      ← HackMD [Text]{prefix:Type} extractions
    │
    ├── <uuid-source-1>/        ← imported page (type: source, read-only)
    │   ├── page.md
    │   ├── meta.ttl             ← includes original-url
    │   ├── relations.ttl
    │   └── annotations.ttl
    │
    └── <uuid-attachment-1>/    ← attachment (type: attachment)
        ├── image.png
        └── meta.ttl             ← includes mime-type
```

### Link Format Comparison

```
OLD (slug-based):              NEW (UUID-based):
─────────────────              ─────────────────
[[Rust]]                   →  [Rust](cerbo://<uuid>)
[[Rust|Rust Lang]]         →  [Rust Lang](cerbo://<uuid>)
![logo](./assets/logo.png) →  ![logo](cerbo://<uuid>)
```

### Object Types (in meta.ttl)

| Type | title | original-url | mime-type | Read-only? |
|------|-------|--------------|-----------|------------|
| `product` | ✓ | ✗ | ✗ | No |
| `source` | ✓ | ✓ | ✗ | **Yes** |
| `attachment` | ✓ | ✗ | ✓ | No |
| `ontology` | ✓ | ✓ | ✗ | No |

## Capabilities

### New Capabilities
- `uuid-object-storage`: UUID-based object storage model with meta.ttl, relations.ttl, annotations.ttl
- `ontology-management`: Ontologies as first-class objects, bundled + importable, prefix mapping via ontology-map.json
- `semantic-annotations`: HackMD-style `[Text]{prefix:Type}` annotations extracted to annotations.ttl
- `vault-init`: `cerbo init` command, `.cerbo/` directory structure, bundled ontologies

### Modified Capabilities
- `page-crud`: Update for UUID-based operations (create, read, write, delete); enforce read-only for `type: source`
- `vault-management`: Replace vault-add with `cerbo init`; update vault structure to use `.cerbo/objects/`
- `page-attachments`: Update for UUID-based attachment objects (separate from pages)
- `wikilink-editing`: Replace `[[Title]]` with `cerbo://<uuid>` markdown links; update editor resolution
- `backlinks`: Update to use `relations.ttl` (cached backlinks) instead of index-based computation
- `slug-resolution`: Remove or replace (slug concept eliminated; title→UUID via index.json)

## Impact

- **Core library** (`core/src/`): Major refactoring of `page.rs`, `vault.rs`, `index.rs`; removal of `slug.rs`
- **CLI commands** (`cli/`): New commands (`init`, `import`, `import-ontology`, `resolve`); modified `page create`, `page read`, `page write`, `page delete`
- **Desktop app** (`src-tauri/`): Editor must resolve titles to UUIDs for `cerbo://` links; render HackMD annotations
- **Dependencies**: Add Turtle/RDF parsing libraries; `oxigraph` crate (optional, for future SPARQL)
- **Breaking change**: Existing vaults cannot migrate (new vaults only, no migration path by design)
