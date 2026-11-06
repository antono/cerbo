# Spec Delta

## Purpose

Stores the user's sidebar layout in the vault: the order of rows inside each
folder and the folders that exist without pages. One optional file,
`.cerbo/tree.toml`, shared by the desktop app and the CLI.

## ADDED Requirements

### Requirement: Tree layout file

A vault MAY contain `.cerbo/tree.toml` with a `pinned` array of folder paths
and an `order` table mapping a folder path (`""` for the root) to an array of
entries. An entry is a page UUID or a child folder name followed by `/`. A
missing file SHALL read as an empty layout. A file that does not parse SHALL
be reported as an error naming the file and SHALL NOT be overwritten. Writes
SHALL be atomic, with keys and pinned paths sorted and deduplicated.

#### Scenario: Vault without the file
- **WHEN** the vault has no `.cerbo/tree.toml`
- **THEN** reading the layout returns no pinned folders and no order lists

#### Scenario: Broken file is not overwritten
- **WHEN** `.cerbo/tree.toml` contains `order = [`
- **AND** the user creates a folder
- **THEN** the operation fails with an error naming `.cerbo/tree.toml`
- **THEN** the file content is unchanged

### Requirement: Display order of a folder's children

The children of a folder SHALL be ordered as follows: first every child named
in the folder's order list, in list order; then every other child, folders
before pages, each group sorted case-insensitively by name or title. Entries
that name no existing child SHALL be skipped; a duplicate entry counts once,
at its first position.

#### Scenario: Listed children first
- **WHEN** `notes` has the folder `rust` and the pages "Alpha" and "Beta"
- **AND** the order list of `notes` is `["<Beta uuid>"]`
- **THEN** the children of `notes` are "Beta", `rust`, "Alpha"

#### Scenario: Stale entries are skipped
- **WHEN** the root's order list names the UUID of a deleted page and then "Home"
- **THEN** the root shows "Home" first and nothing for the deleted page

### Requirement: Pinned folders

A pinned folder path SHALL exist in the tree, with its ancestors, whether or
not a page lives under it. Creating a folder SHALL pin it. Creating a folder
that already exists (pinned or derived from page paths) SHALL fail with
"folder already exists: <path>". Folder paths SHALL follow the
`virtualPath` rules.

#### Scenario: Create an empty folder
- **WHEN** the user creates the folder `projects/2026`
- **THEN** `projects/2026` is pinned in `.cerbo/tree.toml`
- **THEN** the tree shows `projects` containing an empty `2026`

#### Scenario: Create an existing folder
- **WHEN** a page lives at `notes`
- **AND** the user creates the folder `notes`
- **THEN** the operation fails with "folder already exists: notes"

#### Scenario: Invalid folder path
- **WHEN** the user creates the folder `a//b`
- **THEN** the operation fails with the `virtualPath` validation error and nothing is written

### Requirement: Folder delete

Deleting a folder SHALL succeed only when no page lives at or under its path.
It SHALL remove pinned paths at or under it, order lists of it and its
descendants, and its entry in the parent's order list. A folder with pages
SHALL be refused with "folder is not empty: <path> (<n> pages)".

#### Scenario: Delete an empty folder
- **WHEN** `inbox` is pinned and empty
- **AND** the user deletes the folder `inbox`
- **THEN** `inbox` is no longer pinned and the root order list no longer names `inbox/`

#### Scenario: Delete a folder with pages
- **WHEN** three pages live under `notes`
- **AND** the user deletes the folder `notes`
- **THEN** the operation fails with "folder is not empty: notes (3 pages)" and nothing is written

### Requirement: Set a folder's order

Setting the order of a folder SHALL replace its order list with the given
entries, dropping entries that name no existing child and duplicates. An
entry that is neither a UUID nor a valid segment followed by `/` SHALL be
rejected before anything is written.

#### Scenario: Reorder the root
- **WHEN** the root has the folder `notes` and the page "Home"
- **AND** the user sets the root order to `["<Home uuid>", "notes/"]`
- **THEN** the root shows "Home" before `notes`

#### Scenario: Unknown entries are pruned
- **WHEN** the user sets the root order to `["gone/", "<Home uuid>"]` and no folder `gone` exists
- **THEN** the stored root order is `["<Home uuid>"]`

### Requirement: Folder rename keeps the layout

Renaming a folder prefix SHALL also rewrite the layout: every pinned path and
order key equal to the old prefix or under it gets the new prefix; the old
folder's entry in its parent's order list is replaced in place when the
parent is unchanged and removed when the folder moves to another parent. A
rename whose prefix matches no page but a pinned folder SHALL rename the
pinned folder.

#### Scenario: Rename keeps the position
- **WHEN** the root order is `["inbox/", "notes/"]`
- **AND** the user renames `notes` to `journal`
- **THEN** the root order is `["inbox/", "journal/"]`

#### Scenario: Rename carries nested order and pins
- **WHEN** `notes/rust` has an order list and `notes/empty` is pinned
- **AND** the user renames `notes` to `archive/notes`
- **THEN** the order list is stored under `archive/notes/rust`
- **THEN** `archive/notes/empty` is pinned

#### Scenario: Rename a pinned empty folder
- **WHEN** `inbox` is pinned and empty
- **AND** the user renames `inbox` to `todo`
- **THEN** `todo` is pinned and `inbox` is not
