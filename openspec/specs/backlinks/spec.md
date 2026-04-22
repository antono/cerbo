# Backlinks

## Purpose
Enable users to discover relationships between pages by listing all incoming links for the current page.

## Requirements

### Requirement: Display backlinks panel
The system SHALL display a backlinks panel for the currently open page, listing all pages in the active vault that contain a wikilink resolving to the current page.

#### Scenario: Page with backlinks
- **WHEN** the user opens a page that is linked from other pages
- **THEN** the backlinks panel lists each linking page by title
- **THEN** each entry is clickable and navigates to that page

#### Scenario: Page with no backlinks
- **WHEN** the user opens a page that no other page links to
- **THEN** the backlinks panel displays an empty state message

### Requirement: Build link index
The system SHALL build a link index for the active vault by parsing all `page.md` files and extracting `[[wikilink]]` occurrences. The index SHALL be stored as a cache at `$XDG_CACHE_HOME/cerbo/<vault-id>/index.json`.

#### Scenario: Build index on vault open (cache missing)
- **WHEN** a vault is opened and no cache file exists
- **THEN** the system scans all `*/page.md` files and builds the index
- **THEN** the index is written to the cache path

#### Scenario: Build index on vault open (cache stale)
- **WHEN** a vault is opened and the cache `builtAt` timestamp is older than the vault's most recent file modification time
- **THEN** the system rebuilds the index from scratch

#### Scenario: Incremental update on page save
- **WHEN** a `page.md` file is modified
- **THEN** the system updates only that page's entry in the index
- **THEN** the backlinks panel refreshes automatically

### Requirement: Compute backlinks on demand
The system SHALL compute backlinks for a given page by inverting the link index at query time — not by storing backlinks in the index.

#### Scenario: Query backlinks for a page
- **WHEN** the frontend requests backlinks for slug `rust-ownership`
- **THEN** the system scans the index for all pages whose `links` array contains `rust-ownership`
- **THEN** returns the list of matching page slugs and titles
