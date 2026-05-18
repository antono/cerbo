## MODIFIED Requirements

### Requirement: Tree projection from page metadata

The system SHALL build the symlink tree by reading each Product or Source object's `meta.ttl` and projecting it to `<vault-root>/cerbo/<virtualPath>/<slug>.md`. Each segment of every page's `cerbo:virtualPath` SHALL be a real directory; pages sharing a `cerbo:virtualPath` SHALL be merged into the same directory. The leaf, named `<cerbo:slug>.md`, SHALL be a symlink. Pages with an empty or missing `cerbo:virtualPath` SHALL be placed directly under `<vault-root>/cerbo/`.

#### Scenario: Single page at root
- **WHEN** a page has `cerbo:slug "home"` and no `cerbo:virtualPath`
- **THEN** a symlink is created at `<vault-root>/cerbo/home.md`

#### Scenario: Nested page
- **WHEN** a page has `cerbo:slug "ownership"` and `cerbo:virtualPath "notes/rust"`
- **THEN** real directories `<vault-root>/cerbo/notes/` and `<vault-root>/cerbo/notes/rust/` exist
- **THEN** a symlink is created at `<vault-root>/cerbo/notes/rust/ownership.md`

#### Scenario: Multiple pages sharing a virtualPath
- **WHEN** page A has `(virtualPath="notes/rust", slug="ownership")` and page B has `(virtualPath="notes/rust", slug="lifetimes")`
- **THEN** `<vault-root>/cerbo/notes/rust/` is a single real directory
- **THEN** both `ownership.md` and `lifetimes.md` exist as symlinks inside it

### Requirement: Skip non-page objects

The system SHALL materialise only objects whose `meta.ttl` declares `type :Product` or `type :Source`. Objects of type `:Ontology` and `:Attachment` SHALL be skipped. Ontology objects exist for internal vocabulary registration; Attachment objects have no `page.md` content for editors to open.

#### Scenario: Ontology object is not symlinked
- **WHEN** the repository contains an ontology object and a page object
- **THEN** the page is symlinked under `<vault-root>/cerbo/`
- **THEN** the ontology object has no corresponding symlink anywhere in `<vault-root>/cerbo/`

#### Scenario: Attachment object is not symlinked
- **WHEN** the repository contains an attachment object and a page object
- **THEN** the page is symlinked under `<vault-root>/cerbo/`
- **THEN** the attachment object has no corresponding symlink anywhere in `<vault-root>/cerbo/`

#### Scenario: Object missing page.md is skipped with warning
- **WHEN** a Product or Source object has no `page.md` file in its object directory
- **THEN** the object is skipped (no symlink created)
- **THEN** a warning is written to stderr identifying the UUID

### Requirement: Symlink target form

The system SHALL set each leaf symlink's target to the page file `<vault-root>/.cerbo/objects/<uuid>/page.md` (NOT to the object directory). The target SHALL be expressed as a relative path computed from the symlink's parent directory.

#### Scenario: Relative target from root leaf
- **WHEN** a symlink is created at `<vault-root>/cerbo/home.md` for object `<uuid>`
- **THEN** its `readlink` value is `../.cerbo/objects/<uuid>/page.md`

#### Scenario: Relative target from nested leaf
- **WHEN** a symlink is created at `<vault-root>/cerbo/notes/rust/ownership.md` for object `<uuid>`
- **THEN** its `readlink` value is `../../../.cerbo/objects/<uuid>/page.md`

#### Scenario: Target is page.md, not the object directory
- **WHEN** any leaf symlink is created
- **THEN** its target SHALL end with `/.cerbo/objects/<uuid>/page.md`
- **THEN** its target SHALL NOT point to the object directory itself

#### Scenario: Repository is portable
- **WHEN** the repository directory is moved (e.g. `mv my-notes /backup/my-notes`)
- **THEN** every leaf symlink continues to resolve correctly without rebuild
