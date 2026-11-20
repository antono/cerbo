# Spec Delta

## Purpose

A view is an alternative sidebar tree over the vault's pages, stored in the
vault as an object of type `View`. It never moves or copies a page.

## ADDED Requirements

### Requirement: View object

A view SHALL be an object in `.cerbo/objects/<uuid>/` whose `meta.ttl` has the
class `schema:ItemList` and a title, and whose tree is the file `view.toml` in
the same directory. A view SHALL be synced, indexed and trashed like any other
object. A view SHALL NOT appear as a page in `page_list` or in any page tree.

#### Scenario: View is not a page
- **WHEN** the vault has 137 pages and one view
- **THEN** `page_list` returns 137 pages

#### Scenario: Deleting a view
- **WHEN** a view is deleted
- **THEN** its object directory moves to the trash and no page changes

### Requirement: View tree file

`view.toml` SHALL hold an ordered list of nodes. A node is one of:
`folder` (a name and child nodes), `page` (a page UUID) or `query` (a name
and a SPARQL SELECT that binds `?page`). Unknown fields SHALL be rejected.
The same page MAY appear any number of times in one view and in several
views. Writes SHALL be atomic. A file that does not parse SHALL be reported
as an error naming the file and SHALL NOT be overwritten.

#### Scenario: Same page twice
- **WHEN** a view lists page "Home" under folder "Start" and under folder "Today"
- **THEN** both rows render and both open the same page

#### Scenario: Broken file is not overwritten
- **WHEN** `view.toml` is invalid TOML
- **AND** an update is requested
- **THEN** the update fails with an error naming the file
- **THEN** the file content is unchanged

### Requirement: Validation on write

Creating or updating a view SHALL fail, writing nothing, when a `page` node
names a UUID that is not an existing page, when a `query` node does not parse
as a SPARQL SELECT, when a folder name is empty or contains `/`, or when the
title is empty.

#### Scenario: Unknown page
- **WHEN** a view is created with a `page` node for a UUID no page has
- **THEN** creation fails naming that UUID and no object is created

### Requirement: Default view

The vault SHALL always offer a built-in default view "All notes" that is not
an object, cannot be edited through the view operations or deleted, and shows
every page as the tree of `desktop-page-tree` does (`virtualPath` and
`.cerbo/tree.toml`).

#### Scenario: Default view cannot be deleted
- **WHEN** deletion of the default view is requested
- **THEN** it fails and the default view is still offered

### Requirement: Dangling page references

When a `page` node names a page that no longer exists, the view SHALL still
read successfully and mark that node as missing. Nothing SHALL rewrite the
view automatically.

#### Scenario: Page deleted after view creation
- **WHEN** a page referenced by a view is deleted
- **THEN** reading the view returns that node flagged missing
- **THEN** the view file is unchanged

### Requirement: Query nodes

A `query` node SHALL be evaluated read-only with the vault's SPARQL engine
each time the view is read. The page IRIs bound to `?page` become its child
page rows, in result order, capped at 137 rows per node with the total count
reported. A query that fails SHALL fail only that node, with its error text.

#### Scenario: Query returns pages
- **WHEN** a query node matches 3 pages
- **THEN** the node has 3 child page rows

#### Scenario: Query error is local
- **WHEN** one query node has a runtime error
- **THEN** the other nodes of the view still render

### Requirement: Membership triples

A view's `meta.ttl` SHALL record `schema:hasPart` for every page its
`view.toml` references directly, kept in step on every write, so that "which
views contain this page" is a SPARQL query.

#### Scenario: Which views contain a page
- **WHEN** a view references page P
- **THEN** a SPARQL query for `?v schema:hasPart <P>` returns that view

### Requirement: Not part of the symlink tree

`cerbo symlink` SHALL ignore views; its tree stays based on `virtualPath`.

#### Scenario: Symlink tree unchanged
- **WHEN** a view exists
- **THEN** `cerbo symlink` produces the same tree as without it
