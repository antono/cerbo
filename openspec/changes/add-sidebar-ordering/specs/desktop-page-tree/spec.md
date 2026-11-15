# Spec Delta

## MODIFIED Requirements

### Requirement: Sidebar renders pages as a tree

The sidebar SHALL render pages as a tree. Each segment of a page's
`virtualPath` is a folder row; the page is a leaf row under the last segment.
Pages with no `virtualPath` are leaves at the root. Folders pinned in the
vault's tree layout (`.cerbo/tree.toml`) also render, with their ancestors,
even when no page lives under them. Within one level, children named in the
layout's order list for that folder come first, in that order; the other
children follow with folders before pages, each group sorted by name
in the default name order of `vault-tree-layout`.

#### Scenario: Nested pages render as nested rows
- **WHEN** the vault has pages at `notes/rust` ("Ownership"), `notes` ("Index") and at the root ("Home"), and no tree layout
- **THEN** the sidebar shows a folder "notes" and a root leaf "Home"
- **THEN** inside "notes" it shows a folder "rust" before the leaf "Index"
- **THEN** inside "rust" it shows the leaf "Ownership"

#### Scenario: An unpinned folder exists only while a page lives under it
- **WHEN** the only page at `drafts` is moved to the root
- **AND** `drafts` is not pinned
- **THEN** the folder "drafts" disappears from the sidebar

#### Scenario: A pinned folder stays when empty
- **WHEN** `inbox` is pinned and no page lives under it
- **THEN** the sidebar shows an empty folder "inbox"

#### Scenario: Stored order wins
- **WHEN** the layout lists the root as `["Home's uuid", "notes/"]`
- **THEN** the root shows "Home" before the folder "notes"

#### Scenario: Unlisted rows follow in default order
- **WHEN** the root's list names only the page "Zeta"
- **AND** the root also has the folder "notes" and the page "Alpha"
- **THEN** the root shows "Zeta", then "notes", then "Alpha"

#### Scenario: Flat vault renders as before
- **WHEN** no page has a `virtualPath` and there is no tree layout
- **THEN** the sidebar shows only leaf rows at the root, sorted by title, with no folder rows

#### Scenario: Page rows keep their actions
- **WHEN** the user hovers a page row
- **THEN** the Rename, Move and Delete actions are visible
- **THEN** clicking the row opens the page, as in the flat list

### Requirement: Tree refreshes after every change

After a move, a reorder, a folder rename, creation or deletion, a page
creation, rename or delete, and whenever the page list is reloaded (vault
open or switch), the sidebar SHALL re-derive the tree from the current page
list and the current tree layout. The active page stays active unless it was
deleted.

#### Scenario: External move is reflected on reload
- **WHEN** another process changes a page's `cerbo:virtualPath` on disk
- **AND** the user switches away from the vault and back
- **THEN** the page row appears under its new folder

#### Scenario: External layout change is reflected on reload
- **WHEN** another process changes `.cerbo/tree.toml`
- **AND** the user switches away from the vault and back
- **THEN** the rows show in the new order

#### Scenario: Unpinned folder vanishes after delete
- **WHEN** the only page in "drafts" is deleted
- **AND** `drafts` is not pinned
- **THEN** the "drafts" row is removed

#### Scenario: Pinned folder stays after delete
- **WHEN** the only page in the pinned folder "inbox" is deleted
- **THEN** the "inbox" row stays, empty

## REMOVED Requirements

### Requirement: Drag and drop moves a page

**Reason**: Replaced by "Drag places a row before, after or inside" in
`desktop-tree-editing`. Dropping a page on a page row no longer does nothing:
it places the page before or after that row. Folder rows are draggable too.

**Migration**: None for vault data. Dropping a page on the middle of a folder
row or on the zone below the tree still moves it into that folder or to the
root; it now also takes the last position there.
