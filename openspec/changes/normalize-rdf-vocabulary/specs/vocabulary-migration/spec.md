# Spec Delta

## Purpose

Rewrites the metadata of existing vaults from Cerbo's legacy private RDF spellings into the standard vocabulary, so old vaults match what new writes produce.

## ADDED Requirements

### Requirement: Migrate vault metadata to the standard vocabulary

`cerbo migrate vocabulary` SHALL rewrite every live object's `meta.ttl` and `annotations.ttl` that uses a legacy form into the standard form. Legacy forms are the terms replaced by the standard vocabulary (`cerbo:type`, the classes `cerbo:Product`/`Source`/`Attachment`/`Ontology`, `cerbo:title`, `cerbo:original-url`, `cerbo:mime-type`, `schema:` bound to `cerbo://ontology/schema/`) and annotations as `cerbo:annotation`/`cerbo:concept`. Other triples SHALL be preserved.

#### Scenario: Legacy meta.ttl is rewritten
- **WHEN** an object's `meta.ttl` contains `cerbo:type cerbo:Product`, `cerbo:title "Index"` and `<cerbo://ontology/schema/dateCreated>`
- **AND** the user runs `cerbo migrate vocabulary`
- **THEN** the file SHALL contain `rdf:type <https://schema.org/NoteDigitalDocument>` and no `cerbo:Product`, plus `<https://schema.org/name> "Index"` and `<https://schema.org/dateCreated>`
- **THEN** the title, slug, virtual path and date values SHALL be unchanged

#### Scenario: Legacy annotation type is rewritten
- **WHEN** a page's `annotations.ttl` types an annotation as `<cerbo://objects/<uuid-schema-org>/Place>` and the map maps `schema` to that ontology object
- **AND** the user runs `cerbo migrate vocabulary`
- **THEN** the page SHALL `schema:mentions` a node typed `<https://schema.org/Place>` with `schema:name` equal to the former `cerbo:concept`
- **THEN** no `cerbo:annotation` or `cerbo:concept` triple SHALL remain

#### Scenario: Annotation whose prefix was lost
- **WHEN** an annotation type is `<cerbo://ontology/Place>` and the page text still contains `[Berlin]{schema:Place}`
- **THEN** the migration SHALL re-derive the type from the page text as `<https://schema.org/Place>`

### Requirement: Migration scope and safety

The command SHALL accept `--vault <id>` to limit the migration to one vault, and SHALL otherwise migrate every registered vault. It SHALL skip trashed objects. Each file SHALL be rewritten with the atomic replacement protocol. An object that cannot be read SHALL be reported with its UUID, SHALL be left untouched, and SHALL make the command exit non-zero after the other objects are processed.

#### Scenario: One vault only
- **WHEN** the user runs `cerbo migrate vocabulary --vault <id>`
- **THEN** only objects of that vault SHALL be rewritten

#### Scenario: Unreadable object
- **WHEN** one of 137 objects has a `meta.ttl` that does not parse
- **THEN** the other 136 objects SHALL be migrated
- **THEN** stderr SHALL name the broken object's UUID
- **THEN** the command SHALL exit non-zero

### Requirement: Dry run

With `--dry-run`, the command SHALL report what it would change and SHALL NOT modify any file.

#### Scenario: Dry run changes nothing
- **WHEN** the user runs `cerbo migrate vocabulary --dry-run` on a legacy vault
- **THEN** no file under `.cerbo/` SHALL be modified
- **THEN** the output SHALL list the number of files that would be rewritten

### Requirement: Migration is idempotent

Running the migration on a vault that is already in the standard form SHALL rewrite no file and SHALL exit zero.

#### Scenario: Second run
- **WHEN** the user runs `cerbo migrate vocabulary` twice
- **THEN** the second run SHALL report zero rewritten files
- **THEN** no file modification time SHALL change during the second run

### Requirement: Migration summary

The command SHALL print a summary with the number of objects scanned, files rewritten, and errors. With `--json` it SHALL print only a JSON object with the numeric fields `scanned`, `rewritten` and `errors`.

#### Scenario: JSON summary
- **WHEN** the user runs `cerbo migrate vocabulary --json`
- **THEN** stdout SHALL be a valid JSON object with numeric `scanned`, `rewritten` and `errors`
