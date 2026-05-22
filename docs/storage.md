# Cerbo Storage Layout (UUID-Based Model)

## Overview

Cerbo uses a UUID-based object storage model. Each vault has a `.cerbo/` directory containing all metadata and object storage. This is a **breaking change** from the previous slug-based model.

## Directory Structure

```
my-vault/
├── .cerbo/                    # Vault metadata directory
│   ├── objects/               # UUID-based object storage
│   │   ├── <uuid-1>/        # Object directory
│   │   │   ├── page.md       # Page content (for Product, Source, Ontology)
│   │   │   ├── meta.ttl      # Object metadata (Turtle RDF)
│   │   │   ├── backrefs.ttl  # Incoming links (backlinks)
│   │   │   ├── annotations.ttl # HackMD annotations
│   │   │   └── <file>       # Attached files (for Attachment type)
│   │   ├── <uuid-2>/
│   │   └── ...
│   ├── index.json              # Title↔UUID mapping
│   ├── ontology-map.json       # Prefix→UUID mapping for ontologies
│   └── ...
├── actual-content.md           # Regular files in vault (not managed by Cerbo)
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

## File Formats

### meta.ttl (Turtle RDF)
```turtle
@prefix : <cerbo://ontology/> .
@prefix schema: <cerbo://objects/<uuid-schema>/ .
@prefix foaf: <cerbo://objects/<uuid-foaf>/ .

<cerbo://objects/<uuid>>
    :type :Product ;
    :title "My Page" ;
    schema:dateCreated "2024-01-01T00:00:00Z"^^xsd:dateTime ;
    schema:dateModified "2024-01-01T00:00:00Z"^^xsd:dateTime ;
    :original-url "https://example.com" ;  # Only for Source type
    :mime-type "image/png" .               # Only for Attachment type
```

### backrefs.ttl (Turtle RDF)
```turtle
@prefix : <cerbo://ontology/> .

<cerbo://objects/<target-uuid>>
    :hasBacklink <cerbo://objects/<source-uuid>> ;
    :hasBacklink <cerbo://objects/<another-source-uuid>> .
```

### annotations.ttl (Turtle RDF)
```turtle
@prefix : <cerbo://ontology/> .

<cerbo://objects/<uuid>>
    :annotation [
        :concept "Bob" ;
        :type <cerbo://objects/<uuid-schema>/Person> ;
        :position "5,10"
    ] ;
    :annotation [
        :concept "Alice" ;
        :type <cerbo://objects/<uuid-foaf>/Person> ;
        :position "15,20"
    ] .
```

### index.json
```json
{
  "title_to_uuid": {
    "My Page": "123e4567-e89b-12d3-a456-426614174000",
    "Another Page": "987fcdeb-51a2-43d1-b234-567890123456"
  },
  "uuid_to_path": {
    "123e4567-e89b-12d3-a456-426614174000": "objects/123e4567-e89b-12d3-a456-426614174000",
    "987fcdeb-51a2-43d1-b234-567890123456": "objects/987fcdeb-51a2-43d1-b234-567890123456"
  }
}
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
| Links | `[[Page Title]]` | `cerbo://<uuid>` |
| Backlinks | Computed on-the-fly | Cached in `backrefs.ttl` |
| Annotations | Not supported | HackMD syntax → `annotations.ttl` |
| Renaming | Updates all links | Updates `backrefs.ttl` only |
| Read-only | Not enforced | Enforced for Source type |

## Breaking Change

This is a **breaking change** by design. Old vaults using slug-based storage are not compatible. Users must create new vaults and manually migrate content if needed.
