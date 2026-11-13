# Backlinks

## Purpose
Enable users to discover relationships between pages by listing all incoming links for the current page, answered by a query against the vault's semantic index, so no backlink data is stored in the vault.

## Requirements

### Requirement: Display backlinks panel
The system SHALL display a backlinks panel for the currently open page, listing all pages in the active vault that link to the current page. The panel SHALL reside within a multi-section right sidebar. Each entry SHALL be identified by `uuid` and `title`; there SHALL be no `slug` field. Each entry SHALL be navigable by UUID.

#### Scenario: Page with backlinks
- **WHEN** the user opens a page that is linked from other pages
- **THEN** the backlinks panel lists each linking page by title
- **THEN** each entry is clickable and calls `openPage(entry.uuid)`
- **THEN** no slug is used to identify or navigate to the source page

#### Scenario: Page with no backlinks
- **WHEN** the user opens a page that no other page links to
- **THEN** the backlinks panel displays an empty state message

### Requirement: Backlinks are queried from the index

The backlinks of an object SHALL be every live page whose current `page.md` links to it, as recorded by `schema:citation` in the vault's semantic index, with each page's UUID and title. A page SHALL appear once however many times it links. The list SHALL be ordered by title, then UUID.

#### Scenario: Page linked from two pages
- **WHEN** pages A and B both link to page T, and A links to it twice
- **THEN** T's backlinks SHALL be exactly A and B, each once, with their titles

#### Scenario: Page linked from nowhere
- **WHEN** no page links to T
- **THEN** T's backlinks SHALL be an empty list

#### Scenario: Linking page moved to trash
- **WHEN** page A links to T and A is deleted
- **THEN** T's backlinks SHALL NOT include A

### Requirement: Saving a page never writes to link targets

Saving a page SHALL write only the page's own files. It SHALL NOT create, modify or recreate any other object, including objects its links point to. Links to UUIDs that are not live objects SHALL be reported as broken, and the save SHALL succeed.

#### Scenario: Page adds link to another page
- **WHEN** user saves page A with a new link to `cerbo://<uuid-b>`
- **THEN** no file of object B SHALL be modified
- **THEN** B's backlinks SHALL include A

#### Scenario: Page removes link to another page
- **WHEN** user saves page A without its former link to B
- **THEN** B's backlinks SHALL NOT include A

#### Scenario: Link to a non-existent object
- **WHEN** user saves a page containing `[B](cerbo://objects/deadbeef-0000-0000-0000-000000000137)` and no such object exists
- **THEN** no directory SHALL be created under `.cerbo/objects/`
- **THEN** the save SHALL succeed and the link SHALL be reported as broken

#### Scenario: Link to a deleted object does not resurrect it
- **WHEN** object `C` has been deleted and a page still linking to `C` is saved
- **THEN** `.cerbo/objects/C/` SHALL NOT be recreated
- **THEN** `C` SHALL NOT become resolvable

### Requirement: CLI lists backlinks

`cerbo page backlinks <uuid>` SHALL print the object's backlinks, one per line as UUID and title. With `--json` it SHALL print only a JSON array of objects with `uuid` and `title`. An unknown UUID SHALL produce a non-zero exit with "Object not found".

#### Scenario: Backlinks as JSON
- **WHEN** the user runs `cerbo page backlinks <uuid> --json`
- **THEN** stdout SHALL be a valid JSON array whose items have `uuid` and `title`

#### Scenario: Unknown object
- **WHEN** the user runs `cerbo page backlinks 00000000-0000-0000-0000-000000000000`
- **THEN** the command SHALL exit non-zero with "Object not found"
