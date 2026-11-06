# Spec Delta

## Purpose

Lets the user shape the desktop sidebar tree directly: drag any row to any
position, move rows with the keyboard or the row menu, drag folders, and
create or delete folders.

## ADDED Requirements

### Requirement: Drag places a row before, after or inside

Page rows and folder rows SHALL be draggable with the pointer. Dropping a row
SHALL move it to the target's folder when it is in another one, then give it
the target position among the siblings. The drop zones and indicators follow
design D7: before or after a row (a line at its edge), or inside a folder (the
folder highlighted).

#### Scenario: Reorder within a folder
- **WHEN** the root shows "Alpha", "Beta", "Gamma"
- **AND** the user drags "Gamma" onto the top quarter of "Alpha"
- **THEN** the root shows "Gamma", "Alpha", "Beta"
- **THEN** the root order is stored in `.cerbo/tree.toml`

#### Scenario: Move and place in one drop
- **WHEN** the user drags the root page "Loose" onto the bottom half of "Ownership" inside `notes`
- **THEN** "Loose" gets the `virtualPath` `notes`
- **THEN** inside `notes` it shows directly after "Ownership"

#### Scenario: Drop inside a folder
- **WHEN** the user drags a page onto the middle of the folder row `notes`
- **THEN** the page moves into `notes` as its last child
- **THEN** `notes` expands

#### Scenario: Drag a folder
- **WHEN** the user drags the folder `rust` (at `notes/rust`) onto the middle of the folder row `archive`
- **THEN** every page under `notes/rust` moves under `archive/rust`
- **THEN** the tree shows `rust` inside `archive`

#### Scenario: A folder cannot go inside itself
- **WHEN** the user drags the folder `notes` over its own child folder `rust`
- **THEN** no drop indicator is shown
- **THEN** releasing the pointer changes nothing

#### Scenario: The drag needs movement
- **WHEN** the user presses on a row and releases it without moving more than 4 px
- **THEN** it acts as a click: a page opens, a folder toggles

#### Scenario: Escape cancels a drag
- **WHEN** the user is dragging a row
- **AND** presses Escape
- **THEN** the drag ends and nothing moves

#### Scenario: Drag works on Wayland
- **WHEN** the desktop app runs on a Wayland session
- **AND** the user drags a page row onto a folder row
- **THEN** the drag shows its indicator and the drop applies

### Requirement: Move a row with the keyboard

With a row focused and no input focused, `Alt+ArrowUp` or `Alt+k` SHALL swap
the row with its previous sibling and `Alt+ArrowDown` or `Alt+j` with its next
sibling, store the new order, and keep focus on the moved row. At either end
of its siblings nothing happens.

#### Scenario: Move a page up
- **WHEN** focus is on "Beta" in the root "Alpha", "Beta"
- **AND** the user presses `Alt+ArrowUp`
- **THEN** the root shows "Beta", "Alpha" and focus stays on "Beta"

#### Scenario: First row does not move up
- **WHEN** focus is on the first row of a folder
- **AND** the user presses `Alt+k`
- **THEN** nothing changes

### Requirement: Row menu order and folder entries

The row context menu SHALL offer "Move up" and "Move down" on every row,
disabled at the ends of its siblings. The folder menu SHALL offer "New page
here", "New folder here", "Rename folder", and "Delete folder"; "Delete
folder" SHALL appear only when no page lives under the folder.

#### Scenario: Move down from the menu
- **WHEN** the user opens the menu of "Alpha" in the root "Alpha", "Beta" and picks "Move down"
- **THEN** the root shows "Beta", "Alpha"

#### Scenario: Delete an empty folder from the menu
- **WHEN** `inbox` is pinned and empty
- **AND** the user picks "Delete folder" in its menu
- **THEN** `inbox` disappears from the tree

#### Scenario: No delete for a folder with pages
- **WHEN** the user opens the menu of a folder that has pages under it
- **THEN** the menu has no "Delete folder" entry

### Requirement: Add menu on empty sidebar space

Right-clicking sidebar space that is not a row SHALL open an "Add" menu with
"New page" and "New folder". "New page" opens the New Page dialog with an
empty path; "New folder" opens the New Folder dialog with an empty path.

#### Scenario: Add a folder from empty space
- **WHEN** the user right-clicks below the last row and picks "New folder"
- **AND** types `inbox` and confirms
- **THEN** an empty folder "inbox" appears at the root

#### Scenario: Add a page from empty space
- **WHEN** the user right-clicks below the last row and picks "New page"
- **THEN** the New Page dialog opens with an empty path

### Requirement: New folder dialog

The New Folder dialog SHALL take a folder path in the shared path field, with
suggestions of existing folders and the `virtualPath` validation. "New folder
here" SHALL prefill `<folder>/`. Confirming creates the pinned folder and
reveals it; a backend error is shown in the dialog.

#### Scenario: New folder inside a folder
- **WHEN** the user picks "New folder here" on `notes`, types `drafts` after the prefilled `notes/` and confirms
- **THEN** an empty folder `notes/drafts` appears inside `notes`

#### Scenario: Existing folder is refused
- **WHEN** the user types `notes` while `notes` exists and confirms
- **THEN** the dialog shows "folder already exists: notes" and stays open
