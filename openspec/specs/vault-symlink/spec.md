# vault-symlink Specification

## Purpose
TBD - created by archiving change add-vault-symlink-command. Update Purpose after archive.
## Requirements
### Requirement: Repository discovery for `cerbo symlink`

The system SHALL discover the cerbo vault by walking up from the current working directory looking for a directory containing a `.cerbo/` subdirectory. The first such directory found is the vault root. If no `.cerbo/` is found before reaching the filesystem root or a mount-point boundary, the command SHALL abort with the message `not a cerbo vault (or any parent up to mount point)` written to stderr and exit non-zero. The command SHALL NOT accept a positional argument for vault selection.

#### Scenario: Run from vault root
- **WHEN** user runs `cerbo symlink` in a directory containing `.cerbo/`
- **THEN** that directory is treated as the vault root
- **THEN** the command proceeds against it

#### Scenario: Run from nested subdirectory
- **WHEN** user runs `cerbo symlink` from `<vault-root>/notes/rust/`
- **THEN** the command walks up, finds `.cerbo/` at `<vault-root>/`
- **THEN** the command proceeds with `<vault-root>/` as the root

#### Scenario: Run outside any cerbo vault
- **WHEN** user runs `cerbo symlink` from a directory with no `.cerbo/` in any ancestor
- **THEN** the command exits non-zero
- **THEN** stderr contains `not a cerbo vault (or any parent up to mount point)`

#### Scenario: Mount-point boundary
- **WHEN** the walk-up would cross a filesystem mount-point boundary before finding `.cerbo/`
- **THEN** the command stops at the boundary
- **THEN** exits as if no repository was found

#### Scenario: Reject positional argument
- **WHEN** user runs `cerbo symlink some-name`
- **THEN** the command exits non-zero with a clap usage error

### Requirement: Fixed output location at `<vault-root>/cerbo/`

The system SHALL materialise the symlink tree at the fixed path `<vault-root>/cerbo/`. The output directory name SHALL be the literal string `cerbo`, independent of the repository's basename. The tree SHALL always live inside the repository (the same directory tree that contains `.cerbo/`).

#### Scenario: Output directory path
- **WHEN** `cerbo symlink` succeeds in a repository at `/home/anton/my-notes/`
- **THEN** the tree is at `/home/anton/my-notes/cerbo/`

#### Scenario: Output name is independent of repository basename
- **WHEN** `cerbo symlink` succeeds in a repository at `/srv/data/`
- **THEN** the tree is at `/srv/data/cerbo/`, not `/srv/data/data/`

### Requirement: Tree projection from page metadata

The system SHALL build the symlink tree by reading each non-ontology object's `meta.ttl` and projecting it to `<vault-root>/cerbo/<virtualPath>/<slug>`. Each segment of every page's `cerbo:virtualPath` SHALL be a real directory; pages sharing a `cerbo:virtualPath` SHALL be merged into the same directory. The leaf, named after `cerbo:slug`, SHALL be a symlink. Pages with an empty or missing `cerbo:virtualPath` SHALL be placed directly under `<vault-root>/cerbo/`.

#### Scenario: Single page at root
- **WHEN** a page has `cerbo:slug "home"` and no `cerbo:virtualPath`
- **THEN** a symlink is created at `<vault-root>/cerbo/home`

#### Scenario: Nested page
- **WHEN** a page has `cerbo:slug "ownership"` and `cerbo:virtualPath "notes/rust"`
- **THEN** real directories `<vault-root>/cerbo/notes/` and `<vault-root>/cerbo/notes/rust/` exist
- **THEN** a symlink is created at `<vault-root>/cerbo/notes/rust/ownership`

#### Scenario: Multiple pages sharing a virtualPath
- **WHEN** page A has `(virtualPath="notes/rust", slug="ownership")` and page B has `(virtualPath="notes/rust", slug="lifetimes")`
- **THEN** `<vault-root>/cerbo/notes/rust/` is a single real directory
- **THEN** both `ownership` and `lifetimes` exist as symlinks inside it

### Requirement: Skip ontology objects

The system SHALL skip objects whose `meta.ttl` declares `type :Ontology` when building the symlink tree. Ontology objects exist for internal vocabulary registration and are not user-authored content.

#### Scenario: Ontology object is not symlinked
- **WHEN** the repository contains an ontology object (e.g. Schema.org, FOAF) and a page object
- **THEN** the page is symlinked under `<vault-root>/cerbo/`
- **THEN** the ontology object has no corresponding symlink anywhere in `<vault-root>/cerbo/`

### Requirement: Symlink target form

The system SHALL set each leaf symlink's target to the object directory `<vault-root>/.cerbo/objects/<uuid>/` (NOT to `page.md` inside it). The target SHALL be expressed as a relative path computed from the symlink's parent directory.

#### Scenario: Relative target from root leaf
- **WHEN** a symlink is created at `<vault-root>/cerbo/home` for object `<uuid>`
- **THEN** its `readlink` value is `../.cerbo/objects/<uuid>/`

#### Scenario: Relative target from nested leaf
- **WHEN** a symlink is created at `<vault-root>/cerbo/notes/rust/ownership` for object `<uuid>`
- **THEN** its `readlink` value is `../../../.cerbo/objects/<uuid>/`

#### Scenario: Target is the object directory, not page.md
- **WHEN** any leaf symlink is created
- **THEN** its target SHALL end with `/.cerbo/objects/<uuid>/`
- **THEN** its target SHALL NOT include `/page.md` or any other intra-object filename

#### Scenario: Repository is portable
- **WHEN** the repository directory is moved (e.g. `mv my-notes /backup/my-notes`)
- **THEN** every leaf symlink continues to resolve correctly without rebuild

### Requirement: Conflict detection and abort

The system SHALL detect two kinds of conflicts before building the tree and SHALL abort the run if any are found, leaving the existing tree on disk untouched: (a) **leaf-vs-leaf** — two or more pages produce the same `<virtualPath>/<slug>`; (b) **dir-vs-leaf** — one page's combined path coincides with another page's `cerbo:virtualPath` segment at the same level (the same path would have to be both a symlink and a real directory).

#### Scenario: Leaf-vs-leaf collision
- **WHEN** page A has `(virtualPath="notes", slug="rust")` and page B has `(virtualPath="notes", slug="rust")`
- **THEN** the command exits non-zero
- **THEN** stderr lists both UUIDs and the colliding path `notes/rust`
- **THEN** `<vault-root>/cerbo/` is unmodified

#### Scenario: Dir-vs-leaf collision
- **WHEN** page A has `(virtualPath="notes", slug="rust")` and page B has `(virtualPath="notes/rust", slug="ownership")`
- **THEN** the command exits non-zero
- **THEN** stderr explains that `notes/rust` would have to be both a symlink and a directory
- **THEN** stderr lists both UUIDs
- **THEN** `<vault-root>/cerbo/` is unmodified

#### Scenario: No conflict
- **WHEN** no two pages collide on combined path
- **THEN** the command proceeds to build the tree

### Requirement: Full rebuild semantics

The system SHALL rebuild the entire symlink tree on every run. There SHALL be no partial or incremental mode. `.cerbo/` is the single source of truth — anything previously materialised that no longer matches the current metadata SHALL be removed.

#### Scenario: Stale slug is removed on rebuild
- **WHEN** a page previously had `cerbo:slug "old-name"` and now has `cerbo:slug "new-name"`
- **AND** `cerbo symlink` is run
- **THEN** no symlink named `old-name` exists under `<vault-root>/cerbo/`
- **THEN** a symlink named `new-name` exists at the corresponding location

#### Scenario: Deleted page leaves no orphan symlink
- **WHEN** an object directory is removed from `.cerbo/objects/`
- **AND** `cerbo symlink` is run
- **THEN** no symlink in `<vault-root>/cerbo/` points to the removed UUID

### Requirement: Atomic two-phase swap

The system SHALL build the new tree in a sibling temp directory `<vault-root>/cerbo.tmp-<pid>/`, then atomically rename it into place. If a previous `<vault-root>/cerbo/` exists, it SHALL first be renamed aside to `<vault-root>/cerbo.gc-<pid>/` before the new tree is renamed in, and finally removed.

#### Scenario: First run (no existing tree)
- **WHEN** `<vault-root>/cerbo/` does not exist
- **AND** `cerbo symlink` succeeds
- **THEN** `<vault-root>/cerbo.tmp-<pid>/` is built and renamed to `<vault-root>/cerbo/`
- **THEN** no `cerbo.tmp-*` or `cerbo.gc-*` siblings remain

#### Scenario: Replace existing tree
- **WHEN** `<vault-root>/cerbo/` already exists and passes the safe-wipe check
- **AND** `cerbo symlink` succeeds
- **THEN** the existing tree is renamed to `<vault-root>/cerbo.gc-<pid>/`
- **THEN** `<vault-root>/cerbo.tmp-<pid>/` is renamed to `<vault-root>/cerbo/`
- **THEN** `<vault-root>/cerbo.gc-<pid>/` is removed
- **THEN** no `cerbo.tmp-*` or `cerbo.gc-*` siblings remain

#### Scenario: Stale tmp/gc siblings from prior crash
- **WHEN** `cerbo symlink` runs and discovers leftover `cerbo.tmp-*` or `cerbo.gc-*` siblings from a crashed prior run
- **THEN** those leftover siblings SHALL be removed before the new run begins (they are unambiguously cerbo-owned by name pattern)

### Requirement: Safe-wipe rule

Before removing or replacing `<vault-root>/cerbo/`, the system SHALL walk it recursively and verify EVERY entry is either a directory containing only safe entries OR a symlink whose canonicalised target lives under `<vault-root>/.cerbo/objects/`. If any other entry is found (regular file, symlink to elsewhere, device node, etc.), the system SHALL abort with an error listing offending paths and SHALL leave the existing tree untouched.

#### Scenario: All entries are this repository's symlinks
- **WHEN** every leaf under `<vault-root>/cerbo/` is a symlink resolving inside `<vault-root>/.cerbo/objects/`
- **THEN** the safe-wipe check passes
- **THEN** the rebuild proceeds

#### Scenario: Regular file in the tree
- **WHEN** any path under `<vault-root>/cerbo/` is a regular file (not a directory or symlink)
- **THEN** the command exits non-zero
- **THEN** stderr lists that path
- **THEN** `<vault-root>/cerbo/` is unmodified

#### Scenario: Symlink to foreign target
- **WHEN** a symlink under `<vault-root>/cerbo/` points outside `<vault-root>/.cerbo/objects/`
- **THEN** the command exits non-zero
- **THEN** stderr lists that symlink and its target
- **THEN** `<vault-root>/cerbo/` is unmodified

#### Scenario: Symlink to another repository's objects
- **WHEN** a symlink under `<vault-root>/cerbo/` points into a DIFFERENT cerbo vault's `.cerbo/objects/`
- **THEN** the command exits non-zero (the safe-wipe check is scoped to this repository's `.cerbo/objects/` only)
- **THEN** `<vault-root>/cerbo/` is unmodified

### Requirement: Slug fallback for pages without `cerbo:slug`

The system SHALL accept pages that have no `cerbo:slug` predicate by deriving a slug at materialisation time from the page's `cerbo:title` using the project slug algorithm (kebab-case ASCII, deunicode-transliterated, lowercase, 1..=80 characters). If the derived slug is empty, the system SHALL fall back to `untitled-<first-8-chars-of-uuid>`.

#### Scenario: Missing slug, derivable from title
- **WHEN** a page has `cerbo:title "Rust Ownership"` and no `cerbo:slug`
- **THEN** the leaf in `<vault-root>/cerbo/` is named `rust-ownership`

#### Scenario: Empty derived slug
- **WHEN** a page's title transliterates to an empty string (e.g. emoji-only title)
- **AND** no explicit `cerbo:slug` is set
- **THEN** the leaf is named `untitled-<first-8-chars-of-uuid>`

