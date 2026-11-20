# Spec Delta

## MODIFIED Requirements

### Requirement: Sidebar renders pages as a tree

The sidebar SHALL render the tree of the active view (see
`desktop-view-switcher`). In the default view "All notes" it SHALL render
pages as before: each segment of a page's `virtualPath` is a folder row, the
page is a leaf under the last segment, pinned folders render even when empty,
and order follows `.cerbo/tree.toml` then the default name order. In any other
view it SHALL render that view's nodes in file order. Both build the same row
model, so keyboard navigation and drag behave identically.

#### Scenario: Default view is unchanged
- **WHEN** "All notes" is active
- **THEN** the tree is the same as before views existed

#### Scenario: Other view shows its own structure
- **WHEN** a view with folder "Flights" containing one page is active
- **THEN** the sidebar shows folder "Flights" with that page, regardless of the page's `virtualPath`
