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
(case-insensitive, locale aware).

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
