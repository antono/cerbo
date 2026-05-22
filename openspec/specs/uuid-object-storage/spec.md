# UUID Object Storage

## Purpose
Define the core UUID-based object storage model for Cerbo vaults, replacing slug-based page storage with `.cerbo/objects/<uuid>/` layout.
## Requirements
### Requirement: Object Storage Layout
The system SHALL store all objects (pages, attachments, ontologies) under `.cerbo/objects/<uuid>/` directory within the vault root. Each object MUST have a `meta.ttl` file. Pages and ontologies MUST have `page.md`. Attachments store their binary file. Objects MAY have `backrefs.ttl` and `annotations.ttl`.

#### Scenario: Create new page object
- **WHEN** user runs `cerbo create "My Page"`
- **THEN** a new UUID is generated (v4)
- **THEN** directory `.cerbo/objects/<uuid>/` is created
- **THEN** `page.md` is created with `# My Page` as content
- **THEN** `meta.ttl` is created with `type: :Page` (or `:Product` - TBD)

#### Scenario: Object directory structure
- **WHEN** listing `.cerbo/objects/<uuid>/` contents
- **THEN** `meta.ttl` MUST exist
- **THEN** `page.md` exists for Page/Source/Ontology types
- **THEN** binary file exists for Attachment type
- **THEN** `backrefs.ttl` exists if object has outgoing links
- **THEN** `annotations.ttl` exists if object has HackMD annotations

### Requirement: Link Format
The system SHALL use `cerbo://<uuid>` format for all internal links in `page.md`. The system SHALL NOT include `/page.md` or `/image.png` suffix. Link type (page vs attachment) is determined by the target object's `meta.ttl` `type:` field.

#### Scenario: Page link in markdown
- **WHEN** user creates link to another page
- **THEN** the markdown link SHALL be `[Page Title](cerbo://<uuid>)`
- **THEN** the link SHALL NOT include `/page.md` suffix

#### Scenario: Attachment link in markdown
- **WHEN** user creates link to an attachment
- **THEN** the markdown link SHALL be `![Alt Text](cerbo://<uuid>)`
- **THEN** the link SHALL NOT include the filename

### Requirement: Meta TTL Structure
The system SHALL store object metadata in `meta.ttl` using Turtle RDF syntax. The file MUST contain: `type:` (`:Page`, `:Source`, `:Attachment`, `:Ontology`), `:title`, `schema:dateCreated`, `schema:dateModified`. Source types MUST include `:original-url`. Attachments MUST include `:mime-type`.

Page and Source objects MAY include two additional Turtle predicates that govern how the object is exposed in the materialised symlink tree (see the `vault-symlink` capability):

- `cerbo:slug` — string, kebab-case ASCII, no `/`, no leading or trailing `-`, 1..=80 characters. Identifies the leaf name in the symlink tree.
- `cerbo:virtualPath` — string, POSIX-style relative path, no leading or trailing `/`, no `.` or `..` segments, no empty segments, no NUL bytes. Identifies the nested directory structure that contains the leaf. An empty string or a missing predicate means the page lives directly at the symlink-tree root.

These two predicates are **independent**: changing one does not constrain the other. The combined value `<virtualPath>/<slug>` SHALL be unique repository-wide. Objects without these predicates remain valid; the symlink command derives `slug` from `cerbo:title` at materialisation time and treats `virtualPath` as empty.

#### Scenario: Page meta.ttl content
- **WHEN** reading `meta.ttl` for a product page
- **THEN** it SHALL contain `type: :Page` (or `:Product`)
- **THEN** it SHALL contain `:title` with the page title
- **THEN** it SHALL contain `schema:dateCreated` and `schema:dateModified` as xsd:dateTime

#### Scenario: Source meta.ttl content
- **WHEN** reading `meta.ttl` for an imported source
- **THEN** it SHALL contain `type: :Source`
- **THEN** it SHALL contain `:original-url` with the source URL
- **THEN** the object SHALL be read-only

#### Scenario: Attachment meta.ttl content
- **WHEN** reading `meta.ttl` for an attachment
- **THEN** it SHALL contain `type: :Attachment`
- **THEN** it SHALL contain `:mime-type` (e.g., "image/png")

#### Scenario: Page meta.ttl with slug and virtualPath
- **WHEN** reading `meta.ttl` for a page that has been assigned `cerbo:slug "ownership"` and `cerbo:virtualPath "notes/rust"`
- **THEN** both predicates SHALL be present as independent Turtle triples
- **THEN** they SHALL NOT be stored as a compound key

#### Scenario: Slug shape constraint
- **WHEN** any `cerbo:slug` value is read or written
- **THEN** the value SHALL be 1..=80 characters, lowercase ASCII kebab-case, with no `/` and no leading or trailing `-`

#### Scenario: virtualPath shape constraint
- **WHEN** any `cerbo:virtualPath` value is read or written
- **THEN** the value SHALL NOT have a leading or trailing `/`
- **THEN** the value SHALL NOT contain `.` or `..` segments
- **THEN** the value SHALL NOT contain empty segments or NUL bytes

#### Scenario: Combined path uniqueness
- **WHEN** two or more Page or Source objects in the same repository each declare a `cerbo:slug` (and optionally a `cerbo:virtualPath`)
- **THEN** their rendered combined paths `<virtualPath>/<slug>` SHALL be distinct

#### Scenario: Backwards compatibility with pages lacking the new predicates
- **WHEN** a page's `meta.ttl` lacks `cerbo:slug` and `cerbo:virtualPath`
- **THEN** the object remains valid for reading and writing
- **THEN** consumers SHALL derive a slug from `cerbo:title` and treat `virtualPath` as empty

### Requirement: Relations TTL Structure
The system SHALL store outgoing links from `page.md` in `backrefs.ttl` using Turtle RDF syntax. Each `cerbo://<uuid>` link in `page.md` SHALL produce a `:linksTo` (for pages/ontologies) or `:usesAttachment` (for attachments) triple. The system SHALL cache backlinks (`:hasBacklink`) from other objects.

#### Scenario: Extract links to backrefs.ttl
- **WHEN** `page.md` contains `[Page](cerbo://<uuid-1>)` and `![img](cerbo://<uuid-2>)`
- **THEN** `backrefs.ttl` SHALL contain `:linksTo <cerbo://<uuid-1>>`
- **THEN** `backrefs.ttl` SHALL contain `:usesAttachment <cerbo://<uuid-2>>`

#### Scenario: Cache backlinks
- **WHEN** another page links to this object
- **THEN** this object's `backrefs.ttl` SHALL contain `:hasBacklink <cerbo://<other-uuid>>`

### Requirement: Annotations TTL Structure
The system SHALL extract HackMD-style `[Text]{prefix:Type}` annotations from `page.md` and store them in `annotations.ttl` using Turtle RDF syntax. Each annotation SHALL be a blank node with `:concept`, `:type`, and `:position` (line:column).

#### Scenario: Extract HackMD annotations
- **WHEN** `page.md` contains `[Berlin]{schema:Place}`
- **THEN** `annotations.ttl` SHALL contain a blank node with `:concept "Berlin"`, `:type schema:Place`

#### Scenario: Position tracking
- **WHEN** annotation starts at line 5, column 10
- **THEN** `annotations.ttl` SHALL contain `:position "5:10"`

