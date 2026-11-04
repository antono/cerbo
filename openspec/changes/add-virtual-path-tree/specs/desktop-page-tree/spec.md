# Spec Delta

## Purpose

Shows the vault's pages in the desktop sidebar as a folder tree derived from
each page's `cerbo:virtualPath`, and lets the user reshape that tree by moving
pages, renaming folders and creating pages inside folders.

## ADDED Requirements

### Requirement: Sidebar renders pages as a tree

The sidebar SHALL render pages as a tree. Each segment of a page's
`virtualPath` is a folder row; the page is a leaf row under the last segment.
Pages with no `virtualPath` are leaves at the root. Within one level, folders
are listed before pages, each group sorted by name (case-insensitive, locale
aware). Folders are derived from the pages present and never stored.

#### Scenario: Nested pages render as nested rows
- **WHEN** the vault has pages at `notes/rust` ("Ownership"), `notes` ("Index") and at the root ("Home")
- **THEN** the sidebar shows a root leaf "Home" and a folder "notes"
- **THEN** inside "notes" it shows a folder "rust" before the leaf "Index"
- **THEN** inside "rust" it shows the leaf "Ownership"

#### Scenario: A folder exists only while a page lives under it
- **WHEN** the only page at `drafts` is moved to the root
- **THEN** the folder "drafts" disappears from the sidebar

#### Scenario: Flat vault renders as before
- **WHEN** no page has a `virtualPath`
- **THEN** the sidebar shows only leaf rows at the root, sorted by title, with no folder rows

#### Scenario: Page rows keep their actions
- **WHEN** the user hovers a page row
- **THEN** the Rename, Move and Delete actions are visible
- **THEN** clicking the row opens the page, as in the flat list

### Requirement: Folders expand and collapse

Each folder row SHALL toggle between expanded and collapsed on click of its
chevron or label. A collapsed folder hides every row beneath it. Expansion
state SHALL be remembered per vault across app restarts. A folder seen for the
first time is expanded. Opening a page by any means (search, link, history,
restore on launch) SHALL expand every folder on its path so the active row is
visible.

#### Scenario: Collapse hides descendants
- **WHEN** the user collapses "notes"
- **THEN** the rows under "notes", including "rust" and its pages, are not rendered

#### Scenario: State survives restart
- **WHEN** the user collapses "notes" and restarts the app on the same vault
- **THEN** "notes" is still collapsed

#### Scenario: State is per vault
- **WHEN** the user collapses "notes" in vault A and switches to vault B that also has a "notes" folder
- **THEN** "notes" in vault B keeps its own state

#### Scenario: Opening a page reveals it
- **WHEN** "notes" is collapsed and the user opens "Ownership" (at `notes/rust`) from global search
- **THEN** "notes" and "rust" are expanded
- **THEN** the "Ownership" row is highlighted as active and scrolled into view

### Requirement: Move page dialog

The sidebar SHALL offer a "Move to…" action on every page row, available from
the hover action, the row's context menu and the `m` key while the sidebar
has focus and no input is focused. It opens a modal dialog with one path
field prefilled with the page's current path. The field suggests existing
folders as the user types. Confirming applies the move; the page row then
appears under the new path and stays selected.

#### Scenario: Move via dialog
- **WHEN** the user opens "Move to…" on "Ownership" (at `notes/rust`), replaces the field with `archive` and confirms
- **THEN** the page's `virtualPath` becomes `archive`
- **THEN** the sidebar shows "Ownership" under the folder "archive", which is expanded
- **THEN** the dialog closes

#### Scenario: Move to the root
- **WHEN** the user clears the field and confirms
- **THEN** the page's `virtualPath` is removed and the row appears at the root

#### Scenario: Suggestions list existing folders
- **WHEN** the vault has folders `notes`, `notes/rust` and `archive` and the user types `no`
- **THEN** the suggestions show `notes` and `notes/rust`, and not `archive`
- **THEN** choosing a suggestion fills the field with it

#### Scenario: New folder is typed, not created
- **WHEN** the user types a path that matches no existing folder, such as `projects/cerbo`, and confirms
- **THEN** the move succeeds and the folders `projects` and `projects/cerbo` appear, containing the page

#### Scenario: Invalid path is blocked in the dialog
- **WHEN** the field contains `notes/` or `a//b`
- **THEN** the dialog shows the validation message under the field
- **THEN** the confirm action is disabled

#### Scenario: Backend error is shown
- **WHEN** the move command fails
- **THEN** the dialog stays open and shows the returned error text

#### Scenario: Keyboard flow
- **WHEN** the dialog is open
- **THEN** `Enter` confirms, `Escape` closes without changes, and `Tab`/arrow keys move through suggestions

### Requirement: Drag and drop moves a page

A page row SHALL be draggable. Dropping it on a folder row sets the page's
`virtualPath` to that folder's path. Dropping it on the empty area below the
tree or on a dedicated root drop zone moves it to the root. The target
highlights while a valid drop hovers it. Dropping a page on its own current
folder, on a page row, or outside any target does nothing.

#### Scenario: Drop on a folder
- **WHEN** the user drags "Home" (root) onto the folder "notes/rust"
- **THEN** the page's `virtualPath` becomes `notes/rust`
- **THEN** the row appears inside "rust"

#### Scenario: Drop on a collapsed folder
- **WHEN** the user drops a page on a collapsed folder
- **THEN** the move applies
- **THEN** the folder expands to show the moved page

#### Scenario: Drop on the root zone
- **WHEN** the user drags "Ownership" (at `notes/rust`) onto the root drop zone
- **THEN** the page's `virtualPath` is removed

#### Scenario: Drop on a page row does nothing
- **WHEN** the user drops a page on another page row
- **THEN** no move happens and no error is shown

#### Scenario: Drop on the current folder does nothing
- **WHEN** the user drops "Ownership" (at `notes/rust`) on the folder "rust"
- **THEN** no command is invoked

### Requirement: Folder actions

Each folder row SHALL offer "Rename folder" and "New page here" from its hover
actions and context menu. "Rename folder" opens a modal with the folder's last
segment prefilled; confirming renames the prefix on every page under the
folder. "New page here" opens the New Page dialog with the path field
prefilled with the folder's path.

#### Scenario: Rename folder
- **WHEN** the user renames the folder "rust" (at `notes/rust`) to "rustlang"
- **THEN** every page whose path is `notes/rust` or starts with `notes/rust/` now starts with `notes/rustlang`
- **THEN** the sidebar shows "rustlang" in place of "rust", expanded if "rust" was expanded

#### Scenario: Rename folder to a path with a slash
- **WHEN** the user renames the folder "rust" (at `notes/rust`) to `lang/rust`
- **THEN** the pages move under `notes/lang/rust`

#### Scenario: Rename folder to an invalid name
- **WHEN** the new name is empty, `..`, or contains `//`
- **THEN** the dialog shows the validation message and confirm is disabled

#### Scenario: New page in folder
- **WHEN** the user chooses "New page here" on "notes/rust" and creates "Lifetimes"
- **THEN** the page is created with `virtualPath "notes/rust"`
- **THEN** it appears inside "rust" and opens in the editor

### Requirement: Tree refreshes after every change

After a move, folder rename, page creation, rename or delete, and whenever the
page list is reloaded (vault open or switch), the sidebar SHALL re-derive the
tree from the current page list. The active page stays active unless it was
deleted.

#### Scenario: External move is reflected on reload
- **WHEN** another process changes a page's `cerbo:virtualPath` on disk
- **AND** the user switches away from the vault and back
- **THEN** the page row appears under its new folder

#### Scenario: Folder vanishes after delete
- **WHEN** the only page in "drafts" is deleted
- **THEN** the "drafts" row is removed
