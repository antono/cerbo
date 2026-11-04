# Spec Delta

## Purpose

Lets a page's `cerbo:virtualPath` be changed after creation, and lets a whole
folder prefix be renamed across the pages that share it, from the CLI and the
desktop app.

## ADDED Requirements

### Requirement: Move page to a virtual path

The system SHALL change a page's `cerbo:virtualPath` in its `meta.ttl` to a
caller-supplied path. The path SHALL pass the same validation as at page
creation. An empty path SHALL remove the predicate (the page moves to the
root). The page's UUID, directory, slug, title and content SHALL NOT change,
and no other object's files SHALL be written.

#### Scenario: Move page into a folder
- **WHEN** a page with no `cerbo:virtualPath` is moved to `notes/rust`
- **THEN** its `meta.ttl` contains `cerbo:virtualPath "notes/rust"`
- **THEN** its `cerbo:slug`, `schema:name` and every other triple are unchanged
- **THEN** `page.md` is byte-for-byte unchanged

#### Scenario: Move page to the root
- **WHEN** a page with `cerbo:virtualPath "notes/rust"` is moved to the empty path
- **THEN** its `meta.ttl` no longer contains a `cerbo:virtualPath` predicate

#### Scenario: Move page to the path it already has
- **WHEN** a page with `cerbo:virtualPath "notes"` is moved to `notes`
- **THEN** the operation succeeds
- **THEN** `meta.ttl` is semantically unchanged

#### Scenario: Reject an invalid path
- **WHEN** a page is moved to `/notes`, `notes/`, `notes//rust`, `notes/../x` or a path containing a NUL byte
- **THEN** the operation fails with a message naming the rule that was broken
- **THEN** `meta.ttl` is not written

#### Scenario: Move a page that does not exist
- **WHEN** the UUID names no live object
- **THEN** the operation fails with an error indicating the page does not exist

#### Scenario: Move a Source page
- **WHEN** a read-only Source (imported) page is moved to `imports`
- **THEN** the operation succeeds, because only `meta.ttl` placement metadata changes
- **THEN** `page.md` is not written

#### Scenario: Only the moved page is written
- **WHEN** a page is moved in a vault that already contains 137 pages
- **THEN** only files inside the moved object's directory are written

### Requirement: Rename a folder prefix

The system SHALL rename a folder by replacing a leading path prefix with a new
prefix on every page whose `cerbo:virtualPath` equals the prefix or starts with
`<prefix>/`. Both prefixes SHALL be non-empty and pass virtual path validation.
Pages are rewritten one at a time, each with its own atomic `meta.ttl` write.
The result SHALL report the number of pages changed.

#### Scenario: Rename a folder with nested folders
- **WHEN** pages have paths `notes`, `notes/rust`, `notes/rust/async` and `notebook`
- **AND** the folder `notes` is renamed to `archive/notes`
- **THEN** those paths become `archive/notes`, `archive/notes/rust`, `archive/notes/rust/async`
- **THEN** the page at `notebook` is unchanged, because `notebook` is not under `notes/`

#### Scenario: Rename a nested folder only
- **WHEN** the folder `notes/rust` is renamed to `notes/rustlang`
- **THEN** a page at `notes/rust` moves to `notes/rustlang`
- **THEN** a page at `notes` is unchanged

#### Scenario: Rename into an existing folder merges them
- **WHEN** pages exist at `drafts` and at `notes`
- **AND** `drafts` is renamed to `notes`
- **THEN** every former `drafts` page now has path `notes`
- **THEN** the operation succeeds; a slug collision inside the merged folder is reported by `cerbo index`, as for any other collision

#### Scenario: Rename a folder nobody lives in
- **WHEN** no page has a path equal to or under the given prefix
- **THEN** the operation succeeds and reports zero pages changed

#### Scenario: Rename with an invalid new prefix
- **WHEN** the new prefix is empty or fails validation
- **THEN** the operation fails before any page is written

#### Scenario: A failing page does not stop the rest
- **WHEN** one page under the prefix has a `meta.ttl` that cannot be written
- **THEN** the other pages under the prefix are still rewritten
- **THEN** the operation reports the failing UUID and exits with an error

### Requirement: Moves are visible to the semantic index

After a move or folder rename, the semantic index SHALL reflect the new
`cerbo:virtualPath` of every affected page before the next query is answered,
without a manual `cerbo index` run.

#### Scenario: SPARQL sees the new path
- **WHEN** a page is moved to `notes/rust`
- **AND** the user runs `SELECT ?p WHERE { ?p cerbo:virtualPath "notes/rust" }`
- **THEN** the moved page is in the result

#### Scenario: Symlink tree follows the move
- **WHEN** a page is moved to `notes/rust` and `cerbo symlink` is run
- **THEN** the page's symlink is at `<vault-root>/cerbo/notes/rust/<slug>.md` and nowhere else

### Requirement: Desktop commands for moving pages and renaming folders

The desktop app SHALL expose a command that moves one page by UUID to a given
path, and a command that renames a folder prefix. Both SHALL return an error
string on failure that the UI can show verbatim.

#### Scenario: Move from the desktop
- **WHEN** the frontend invokes the move command with a page UUID and the path `notes/rust`
- **THEN** the page's `meta.ttl` is updated as in "Move page to a virtual path"
- **THEN** the command resolves without a value

#### Scenario: Invalid path from the desktop
- **WHEN** the frontend invokes the move command with the path `notes/`
- **THEN** the command rejects with the validation message
- **THEN** nothing is written
