# Page Metadata Index

## Purpose

Defines requirements for the `cerbo index` CLI command, which rebuilds metadata files (`backrefs.ttl` and `annotations.ttl`) by re-parsing page content. Supports scoped operation across all vaults, a single vault, or a single page.
## Requirements
### Requirement: Index page metadata with scope control

The system SHALL provide a CLI command `cerbo index` that indexes `backrefs.ttl` and `annotations.ttl` files by re-parsing page content, with support for three scopes: all pages (default), single vault, or single page.

#### Scenario: Index all pages across all vaults
- **WHEN** user runs `cerbo index` with no arguments
- **THEN** system scans all vaults, processes all pages, and regenerates backrefs and annotations for each page

#### Scenario: Index specific vault by UUID
- **WHEN** user runs `cerbo index --vault <vault-uuid>`
- **THEN** system processes only pages within the specified vault UUID

#### Scenario: Index single page by UUID
- **WHEN** user runs `cerbo index --page <page-uuid>`
- **THEN** system processes only the specified page, updating its outgoing links and annotations

#### Scenario: Conflicting scope flags
- **WHEN** user runs `cerbo index --vault <vault-uuid> --page <page-uuid>`
- **THEN** system exits with error: "Cannot specify both --vault and --page"

#### Scenario: Handle parsing errors gracefully
- **WHEN** a page.md file is corrupted or unparseable
- **THEN** system logs the error, continues processing remaining pages, and exits with non-zero status code

### Requirement: Extract and regenerate backref metadata

The system SHALL parse each `page.md` file for `cerbo://<uuid>` links and update the corresponding `backrefs.ttl` files for each referenced page.

#### Scenario: Page with outgoing links
- **WHEN** page A contains `cerbo://uuid-b` and `cerbo://uuid-c`
- **THEN** system adds page A's UUID to `backrefs.ttl` in both page B and page C's object directories

#### Scenario: Page with no links
- **WHEN** page contains no `cerbo://` links
- **THEN** no backref files are modified for other pages (but this page's backrefs may still be updated by other pages linking to it)

#### Scenario: Broken link reference
- **WHEN** page contains `cerbo://nonexistent-uuid`
- **THEN** system logs a warning but continues processing (orphaned link)

### Requirement: Extract and regenerate annotation metadata

The system SHALL parse each `page.md` file for `[text]{prefix:Type}` annotations and regenerate the `annotations.ttl` file for that page.

#### Scenario: Page with annotations
- **WHEN** page contains `[John Doe]{foaf:Person}` and `[ACME Corp]{schema:Organization}`
- **THEN** system writes `annotations.ttl` with both annotations, including line/column positions

#### Scenario: Page with no annotations
- **WHEN** page contains no annotation syntax
- **THEN** `annotations.ttl` is either removed or written as empty (depending on implementation)

### Requirement: Provide progress feedback

The system SHALL display progress information during indexing operations for user feedback.

#### Scenario: Progress updates
- **WHEN** indexing a vault with 500 pages
- **THEN** system displays periodic updates like "Processing 100/500 pages..." to stderr

#### Scenario: Summary on completion
- **WHEN** indexing completes
- **THEN** system outputs summary statistics to stdout: total pages processed, links found, annotations extracted, errors encountered

#### Scenario: Error reporting
- **WHEN** errors occur during indexing
- **THEN** system logs error details (file path, error message) to stderr and exits with non-zero code

### Requirement: Idempotent operation

The system SHALL produce identical results when run multiple times on the same vault state.

#### Scenario: Multiple runs on all pages
- **WHEN** user runs `cerbo index` twice on unchanged vault
- **THEN** second run produces identical backrefs.ttl and annotations.ttl files as first run

#### Scenario: Multiple runs on single page
- **WHEN** user runs `cerbo index --page <uuid>` twice without changing the page
- **THEN** second run produces identical backrefs.ttl and annotations.ttl as first run

#### Scenario: Incremental changes
- **WHEN** user adds a link to page A, runs index, then removes the link and runs index again
- **THEN** final state matches original state (backref removed from target page)

### Requirement: Backfill missing slugs

The system SHALL detect Page and Source objects whose `meta.ttl` lacks a `cerbo:slug` predicate and write a derived slug back to `meta.ttl`, computed from `cerbo:title` using the project slug algorithm (kebab-case ASCII, deunicode-transliterated, lowercase, 1..=80 characters, fallback to `untitled-<first-8-chars-of-uuid>` for empty results). The system SHALL emit one log line per backfilled object to stderr. A `--no-backfill-slug` flag SHALL disable this behavior, making `cerbo index` read-only with respect to slugs.

#### Scenario: Backfill a missing slug
- **WHEN** an object has `cerbo:title "Rust Ownership"` and no `cerbo:slug`
- **AND** `cerbo index` is run without `--no-backfill-slug`
- **THEN** `meta.ttl` is updated to include `cerbo:slug "rust-ownership"`
- **THEN** stderr logs a line identifying the UUID and the new slug

#### Scenario: Do not backfill when flag is set
- **WHEN** an object has no `cerbo:slug`
- **AND** `cerbo index` is run with `--no-backfill-slug`
- **THEN** `meta.ttl` SHALL NOT be modified
- **THEN** stderr logs that the slug is missing (so users can fix it manually)

#### Scenario: Skip when slug already present
- **WHEN** an object already has `cerbo:slug "custom-name"`
- **AND** `cerbo index` is run
- **THEN** `meta.ttl` SHALL NOT be modified for that object's slug

#### Scenario: Backfill skips ontology objects
- **WHEN** an object has `type :Ontology`
- **AND** `cerbo index` is run
- **THEN** `meta.ttl` for the ontology SHALL NOT receive a `cerbo:slug` (ontologies are not symlinked)

### Requirement: Validate virtualPath shape

The system SHALL validate `cerbo:virtualPath` on every Page and Source object. Invalid values SHALL be reported to stderr with the offending UUID and the specific reason. The system SHALL NOT auto-fix invalid `cerbo:virtualPath` values; the user must repair them by editing `meta.ttl` directly.

#### Scenario: Detect leading slash
- **WHEN** an object's `cerbo:virtualPath` begins with `/`
- **THEN** stderr reports the UUID and the reason "virtualPath must not begin with `/`"
- **THEN** `meta.ttl` SHALL NOT be modified

#### Scenario: Detect `..` segment
- **WHEN** an object's `cerbo:virtualPath` contains a `..` segment
- **THEN** stderr reports the UUID and the reason "virtualPath must not contain `..` segments"
- **THEN** `meta.ttl` SHALL NOT be modified

#### Scenario: Detect empty segment
- **WHEN** an object's `cerbo:virtualPath` contains an empty segment (e.g. `notes//rust`)
- **THEN** stderr reports the UUID and the reason "virtualPath must not contain empty segments"
- **THEN** `meta.ttl` SHALL NOT be modified

#### Scenario: Valid virtualPath passes
- **WHEN** an object's `cerbo:virtualPath` is `notes/rust`
- **THEN** validation produces no error for that object

### Requirement: Detect combined-path collisions

The system SHALL compute the rendered path `<virtualPath>/<slug>` for every non-ontology object and detect collisions of two kinds: (a) two or more objects producing the same combined path (leaf-vs-leaf), and (b) one object's combined path coinciding with another object's `cerbo:virtualPath` segment at the same level (dir-vs-leaf). Collisions SHALL be reported to stderr with the offending paths and UUIDs. The system SHALL exit non-zero if any collisions are detected.

#### Scenario: Leaf-vs-leaf collision
- **WHEN** two objects produce the combined path `notes/rust/ownership`
- **THEN** stderr reports the colliding path and both UUIDs
- **THEN** the command exits non-zero

#### Scenario: Dir-vs-leaf collision
- **WHEN** object A has combined path `notes/rust` and object B has combined path `notes/rust/ownership`
- **THEN** stderr reports that `notes/rust` would have to be both a symlink and a directory
- **THEN** stderr reports both UUIDs
- **THEN** the command exits non-zero

#### Scenario: No collision
- **WHEN** no two objects' combined paths collide
- **THEN** the collision check exits with status zero (subject to other index errors)

