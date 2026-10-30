# Spec Delta

## MODIFIED Requirements

### Requirement: Object Storage Layout
The system SHALL store all objects (pages, attachments, ontologies) under
`.cerbo/objects/<uuid>/` directory within the vault root. Each object MUST have a `meta.ttl`
file. Pages and ontologies MUST have `page.md`. Attachments store their binary file. Objects
MAY have `backrefs.ttl` and `annotations.ttl`.

An object's class SHALL be exactly one of four schema.org classes, which also decides its
behaviour: `schema:NoteDigitalDocument` for a user-created page (formerly `:Product`),
`schema:WebPage` for imported read-only content (formerly `:Source`), `schema:MediaObject`
for a binary file (formerly `:Attachment`), and `schema:DefinedTermSet` for an imported
ontology (formerly `:Ontology`). In the rest of the specifications, "Product", "Source",
"Attachment" and "Ontology" name these four classes.

#### Scenario: Create new page object
- **WHEN** user runs `cerbo create "My Page"`
- **THEN** a new UUID is generated (v4)
- **THEN** directory `.cerbo/objects/<uuid>/` is created
- **THEN** `page.md` is created with `# My Page` as content
- **THEN** `meta.ttl` is created recording the class `schema:NoteDigitalDocument`

#### Scenario: Object directory structure
- **WHEN** listing `.cerbo/objects/<uuid>/` contents
- **THEN** `meta.ttl` MUST exist
- **THEN** `page.md` exists for Product/Source/Ontology types
- **THEN** binary file exists for Attachment type
- **THEN** `backrefs.ttl` exists once anything has linked to the object
- **THEN** `annotations.ttl` exists if object has HackMD annotations

### Requirement: Meta TTL Structure
The system SHALL store object metadata in `meta.ttl` using Turtle RDF syntax. The file MUST contain, with `schema:` meaning `https://schema.org/`: exactly one object class as `rdf:type` (see "Object Storage Layout"); `schema:name` (the title); `schema:dateCreated` and `schema:dateModified`. Sources MUST include `schema:url`. Attachments MUST include `schema:encodingFormat`. Other `rdf:type` values MAY be present and SHALL NOT change behaviour.

When reading, the system SHALL also accept the legacy forms `cerbo:type`, `cerbo:title`, `cerbo:original-url`, `cerbo:mime-type` and `schema:` bound to `cerbo://ontology/schema/`, and SHALL map the legacy classes `cerbo:Product`, `cerbo:Source`, `cerbo:Attachment` and `cerbo:Ontology` to their schema.org classes, so vaults written before the vocabulary change stay readable. When writing, the system SHALL use only the standard forms.

Page and Source objects MAY include two additional Turtle predicates that govern how the object is exposed in the materialised symlink tree (see the `vault-symlink` capability):

- `cerbo:slug` — string, kebab-case ASCII, no `/`, no leading or trailing `-`, 1..=80 characters. Identifies the leaf name in the symlink tree.
- `cerbo:virtualPath` — string, POSIX-style relative path, no leading or trailing `/`, no `.` or `..` segments, no empty segments, no NUL bytes. Identifies the nested directory structure that contains the leaf. An empty string or a missing predicate means the page lives directly at the symlink-tree root.

These two predicates are **independent**: changing one does not constrain the other. The combined value `<virtualPath>/<slug>` SHALL be unique repository-wide. Objects without these predicates remain valid; the symlink command derives `slug` from the title (`schema:name`) at materialisation time and treats `virtualPath` as empty.

#### Scenario: Page meta.ttl content
- **WHEN** reading `meta.ttl` for a product page
- **THEN** it SHALL contain `rdf:type <https://schema.org/NoteDigitalDocument>`
- **THEN** it SHALL NOT contain any `cerbo://ontology/` class
- **THEN** it SHALL contain `<https://schema.org/name>` with the page title
- **THEN** it SHALL contain `<https://schema.org/dateCreated>` and `<https://schema.org/dateModified>` as xsd:dateTime

#### Scenario: Type is visible to standard RDF tools
- **WHEN** a page's `meta.ttl` is loaded into any SPARQL engine
- **THEN** `SELECT ?s ?n WHERE { ?s a <https://schema.org/NoteDigitalDocument> ; <https://schema.org/name> ?n }` SHALL return the page's IRI and title

#### Scenario: Legacy meta.ttl is still readable
- **WHEN** `meta.ttl` contains `cerbo:type cerbo:Product`, `cerbo:title "Index"` and dates under `cerbo://ontology/schema/`
- **THEN** the object SHALL be read with type Product, title "Index" and the stored dates
- **THEN** the next write of that object SHALL use only the standard forms, with `rdf:type schema:NoteDigitalDocument` instead of `cerbo:Product`

#### Scenario: Ambiguous object class
- **WHEN** a `meta.ttl` has both `rdf:type schema:NoteDigitalDocument` and `rdf:type schema:WebPage`
- **THEN** reading the object SHALL fail with an error naming the UUID and both classes
- **THEN** the object SHALL NOT be written or treated as either class

#### Scenario: Extra unrelated class
- **WHEN** a user adds `rdf:type <https://schema.org/Recipe>` to a note's `meta.ttl` by hand
- **THEN** the note SHALL keep behaving as a note

#### Scenario: Source meta.ttl content
- **WHEN** reading `meta.ttl` for an imported source
- **THEN** it SHALL contain `rdf:type <https://schema.org/WebPage>`
- **THEN** it SHALL contain `<https://schema.org/url>` with the source URL
- **THEN** the object SHALL be read-only

#### Scenario: Attachment meta.ttl content
- **WHEN** reading `meta.ttl` for an attachment
- **THEN** it SHALL contain `rdf:type <https://schema.org/MediaObject>`
- **THEN** it SHALL contain `<https://schema.org/encodingFormat>` (e.g., "image/png")

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
- **THEN** consumers SHALL derive a slug from the title and treat `virtualPath` as empty

### Requirement: Annotations TTL Structure
The system SHALL extract HackMD-style `[Text]{prefix:Type}` annotations from `page.md` and store them in `annotations.ttl` using Turtle RDF syntax. Each annotation SHALL be a blank node the page mentions (`<page> schema:mentions _:m`), with `schema:name` (the annotated text), `rdf:type` (the resolved type IRI, see `semantic-annotations`) and `cerbo:position` (`line:column`).

#### Scenario: Extract HackMD annotations
- **WHEN** `page.md` contains `[Berlin]{schema:Place}`
- **THEN** `annotations.ttl` SHALL contain `<page> schema:mentions _:m` where `_:m` has `schema:name "Berlin"` and `rdf:type <https://schema.org/Place>`

#### Scenario: Position tracking
- **WHEN** annotation starts at line 5, column 10
- **THEN** `annotations.ttl` SHALL contain `cerbo:position "5:10"`
