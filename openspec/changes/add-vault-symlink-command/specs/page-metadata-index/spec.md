## ADDED Requirements

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
