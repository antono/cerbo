# UUID Object Storage

## Purpose
Define the core UUID-based object storage model for Cerbo vaults, replacing slug-based page storage with `.cerbo/objects/<uuid>/` layout.
## Requirements

### Requirement: Object Storage Layout
The system SHALL store all objects (pages, attachments, ontologies) under
`.cerbo/objects/<uuid>/` directory within the vault root. Each object MUST have a `meta.ttl`
file. Pages and ontologies MUST have `page.md`. Attachments store their binary file. Objects
MAY have `backrefs.ttl` and `annotations.ttl`.

An object's type SHALL be recorded as one of exactly four values: `:Product` for a
user-created page, `:Source` for imported read-only content, `:Attachment` for a binary file,
and `:Ontology` for an imported ontology. `:Page` is not a type; pages are `:Product`.

#### Scenario: Create new page object
- **WHEN** user runs `cerbo create "My Page"`
- **THEN** a new UUID is generated (v4)
- **THEN** directory `.cerbo/objects/<uuid>/` is created
- **THEN** `page.md` is created with `# My Page` as content
- **THEN** `meta.ttl` is created recording the type `:Product`

#### Scenario: Object directory structure
- **WHEN** listing `.cerbo/objects/<uuid>/` contents
- **THEN** `meta.ttl` MUST exist
- **THEN** `page.md` exists for Product/Source/Ontology types
- **THEN** binary file exists for Attachment type
- **THEN** `backrefs.ttl` exists once anything has linked to the object
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

### Requirement: Annotations TTL Structure
The system SHALL extract HackMD-style `[Text]{prefix:Type}` annotations from `page.md` and store them in `annotations.ttl` using Turtle RDF syntax. Each annotation SHALL be a blank node with `:concept`, `:type`, and `:position` (line:column).

#### Scenario: Extract HackMD annotations
- **WHEN** `page.md` contains `[Berlin]{schema:Place}`
- **THEN** `annotations.ttl` SHALL contain a blank node with `:concept "Berlin"`, `:type schema:Place`

#### Scenario: Position tracking
- **WHEN** annotation starts at line 5, column 10
- **THEN** `annotations.ttl` SHALL contain `:position "5:10"`

### Requirement: Object metadata is valid Turtle
`meta.ttl` SHALL be a syntactically valid Turtle document that any conformant RDF parser can
read. Its subject SHALL be the object's own IRI, derived from its UUID. Every prefix used in
the document SHALL be declared in it, and the document SHALL contain exactly one statement
terminator per statement.

#### Scenario: Metadata parses with a conformant Turtle parser
- **WHEN** any object's `meta.ttl` is parsed by a conformant Turtle parser
- **THEN** parsing SHALL succeed with no syntax errors

#### Scenario: Subject is the object's own IRI
- **WHEN** the object with UUID `f7e435db-9740-4a2a-8e57-b439c4f8bb18` writes its metadata
- **THEN** the subject SHALL be `<cerbo://objects/f7e435db-9740-4a2a-8e57-b439c4f8bb18>`
- **THEN** two distinct objects SHALL NOT share a subject IRI

#### Scenario: All prefixes used are declared
- **WHEN** `meta.ttl` uses a prefixed name
- **THEN** that prefix SHALL be declared in the same document

### Requirement: Metadata literals are escaped and round-trip losslessly
All literal values written into `meta.ttl` SHALL be escaped so that any character a user can
type in a title is preserved exactly. Writing a value and reading it back SHALL yield the
original value, and re-serialising an unchanged object SHALL produce byte-identical output.

#### Scenario: Title containing a double quote
- **WHEN** a page title is `He said "hi"`
- **THEN** reading the title back SHALL yield exactly `He said "hi"`

#### Scenario: Title containing a newline
- **WHEN** a page title contains a newline character
- **THEN** reading the title back SHALL yield the title including the newline
- **THEN** the metadata document SHALL remain valid Turtle

#### Scenario: Title containing a backslash and non-ASCII text
- **WHEN** a page title is `C:\temp — заметки 🎉`
- **THEN** reading the title back SHALL yield exactly `C:\temp — заметки 🎉`

#### Scenario: Rewriting unchanged metadata is byte-stable
- **WHEN** an object's metadata is read and written back with no field changed
- **THEN** the resulting file SHALL be byte-identical to the original

### Requirement: Metadata fields are parsed by position, not substring search
Object metadata SHALL be interpreted by parsing the document's triples. A value's text SHALL
never be interpreted as a predicate, a type, or any other structural element.

#### Scenario: Title that looks like a type assertion
- **WHEN** a page title is `RDF :type :Source notes`
- **THEN** the object's type SHALL remain its actual type, not `Source`
- **THEN** the title SHALL read back as exactly `RDF :type :Source notes`
- **THEN** the page SHALL remain writable and deletable

#### Scenario: Title that looks like an ontology type assertion
- **WHEN** a page title is `Notes on :type :Ontology usage`
- **THEN** the object SHALL continue to appear in page listings and the symlink tree
- **THEN** the object SHALL continue to be indexed

#### Scenario: Existing damaged metadata recovers without rewriting files
- **WHEN** a vault already contains an object whose title injected a type assertion
- **THEN** reading it after this change SHALL report its correct type and full title
- **THEN** recovery SHALL NOT require rewriting the file or running a migration
