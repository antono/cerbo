# UUID Object Storage

## ADDED Requirements

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

## REMOVED Requirements

### Requirement: Index JSON as part of the object store contract
**Reason**: `.cerbo/index.json` has no readers anywhere in the codebase. Its maps are never
consulted for resolution, are never updated on delete or rename, and are therefore already
permanently stale. It is rewritten in full on every object creation, making page creation
O(N) in vault size, and it deletes itself and returns an empty map on any parse error, so a
single corrupt byte is unrecoverable total loss. `uuid_to_path`'s value is derivable from its
own key.

**Migration**: No user action is required. An existing `.cerbo/index.json` is ignored and may
be deleted by hand at any time; it is never read, so its presence or absence changes no
behaviour. Title-to-UUID resolution is specified in the `slug-resolution` capability as a
scan of object metadata, which is what the implementation already does.
