# Spec Delta

## ADDED Requirements

### Requirement: Command-line folder list

The `cerbo` CLI SHALL provide `cerbo folder list [--json]` printing the
ordered tree: two spaces of indent per level, folders as `name/`, pages as
`Title  <uuid>`. With `--json` it prints nested nodes
`{"kind": "folder", "name", "path", "pinned", "children"}` and
`{"kind": "page", "uuid", "title"}`.

#### Scenario: List the ordered tree
- **WHEN** the root order is `["<Home uuid>", "notes/"]` and "Ownership" lives at `notes`
- **AND** user runs `cerbo folder list`
- **THEN** stdout is `Home  <Home uuid>`, then `notes/`, then `  Ownership  <uuid>`

### Requirement: Command-line folder create

The `cerbo` CLI SHALL provide `cerbo folder create <PATH> [--json]` to pin a
new folder. On success it prints `Created folder <path>`; errors go to stderr
with a non-zero exit code.

#### Scenario: Create a folder
- **WHEN** user runs `cerbo folder create inbox`
- **THEN** stdout is `Created folder inbox`
- **THEN** `cerbo folder list` shows `inbox/`

### Requirement: Command-line folder delete

The `cerbo` CLI SHALL provide `cerbo folder delete <PATH> [--json]` to delete
an empty folder. On success it prints `Deleted folder <path>`; a folder with
pages is refused on stderr with a non-zero exit code.

#### Scenario: Delete an empty folder
- **WHEN** `inbox` is pinned and empty
- **AND** user runs `cerbo folder delete inbox`
- **THEN** stdout is `Deleted folder inbox` and `inbox/` is gone from `cerbo folder list`

#### Scenario: Delete a non-empty folder
- **WHEN** a page lives at `notes`
- **AND** user runs `cerbo folder delete notes`
- **THEN** the command exits non-zero with "folder is not empty: notes (1 pages)" on stderr

### Requirement: Command-line folder order

The `cerbo` CLI SHALL provide `cerbo folder order <PATH|--root> <ENTRY>...
[--json]` to set the order of a folder's children. An entry is a page UUID or
a child folder name, with or without a trailing `/`. On success it prints
`Ordered <path|root> (<n> entries)`.

#### Scenario: Order with folder names
- **WHEN** user runs `cerbo folder order --root notes <Home uuid>`
- **THEN** stdout is `Ordered root (2 entries)`
- **THEN** the stored root order is `["notes/", "<Home uuid>"]`
