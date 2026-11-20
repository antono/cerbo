# Spec Delta

## ADDED Requirements

### Requirement: View selector above the tree

The sidebar SHALL show a dropdown above the tree listing "All notes" first and
then every view by title. Choosing an entry SHALL render that view's tree.

#### Scenario: Switching view
- **WHEN** the user opens the dropdown and picks "Berlin trip"
- **THEN** the tree shows the nodes of that view
- **THEN** the dropdown shows "Berlin trip"

### Requirement: Remembered choice

The active view SHALL be remembered per vault in `ui.toml`. When the stored
view no longer exists, the app SHALL fall back to "All notes" and drop the
stored value.

#### Scenario: Restart
- **WHEN** the user closes the app on "Berlin trip" and reopens the vault
- **THEN** "Berlin trip" is active

#### Scenario: View deleted
- **WHEN** the stored view was deleted
- **THEN** "All notes" is active

### Requirement: Editing inside a view

In a view, drag, keyboard and menu reordering SHALL change only that view's
`view.toml`. A page's `virtualPath` and `.cerbo/tree.toml` SHALL NOT change.
"Add to view" in a page's context menu SHALL add a reference to a chosen
view; it works from any view, including "All notes". "Remove from view"
removes a node and never deletes a page.

#### Scenario: Reordering does not move pages
- **WHEN** the user reorders two pages in a view
- **THEN** both keep their `virtualPath`
- **THEN** "All notes" shows the same tree as before

#### Scenario: Remove from view
- **WHEN** the user picks "Remove from view" on a page row
- **THEN** the row disappears from that view and the page still exists

### Requirement: Special rows

A missing-page node SHALL render muted with the label "Missing page" and a
"Remove from view" action. A query node whose evaluation failed SHALL render
its error inline. A query node capped at 137 rows SHALL show how many more
exist.

#### Scenario: Missing page row
- **WHEN** a view references a deleted page
- **THEN** the row is muted and offers "Remove from view"

### Requirement: Keyboard behaviour

The flat-row keyboard navigation of `desktop-page-tree` SHALL work in every
view, and next/previous page SHALL follow the active view's visible page rows.

#### Scenario: Next page in a view
- **WHEN** a view is active and the user triggers next page
- **THEN** the page after the current one in that view's order opens
