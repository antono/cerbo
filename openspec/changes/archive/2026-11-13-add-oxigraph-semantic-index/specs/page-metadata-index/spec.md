# Spec Delta

## MODIFIED Requirements

### Requirement: Index page metadata with scope control

The system SHALL provide a CLI command `cerbo index` that regenerates `annotations.ttl` files by re-parsing page content and updates the semantic index, with support for three scopes: all pages (default), single vault, or single page. It SHALL NOT write `backrefs.ttl` files.

#### Scenario: Index all pages across all vaults
- **WHEN** user runs `cerbo index` with no arguments
- **THEN** system scans all vaults, processes all pages, and regenerates annotations and the semantic index for each page

#### Scenario: Index specific vault by UUID
- **WHEN** user runs `cerbo index --vault <vault-uuid>`
- **THEN** system processes only pages within the specified vault UUID

#### Scenario: Index single page by UUID
- **WHEN** user runs `cerbo index --page <page-uuid>`
- **THEN** system processes only the specified page, updating its outgoing links and annotations

#### Scenario: Conflicting scope flags
- **WHEN** user runs `cerbo index --vault <vault-uuid> --page <page-uuid>`
- **THEN** system exits with error: "Cannot specify both --vault and --page"

#### Scenario: Handle parsing errors gracefully
- **WHEN** a page.md file is corrupted or unparseable
- **THEN** system logs the error, continues processing remaining pages, and exits with non-zero status code

### Requirement: Idempotent operation

The system SHALL produce identical results when run multiple times on the same vault state.

#### Scenario: Multiple runs on all pages
- **WHEN** user runs `cerbo index` twice on unchanged vault
- **THEN** second run produces identical annotations.ttl files and an index answering every query identically

#### Scenario: Multiple runs on single page
- **WHEN** user runs `cerbo index --page <uuid>` twice without changing the page
- **THEN** second run produces identical annotations.ttl and index content as first run

#### Scenario: Incremental changes
- **WHEN** user adds a link to page A, runs index, then removes the link and runs index again
- **THEN** final state matches original state (the target page no longer lists A as a backlink)

## REMOVED Requirements

### Requirement: Extract and regenerate backref metadata
**Reason**: Backlinks are no longer stored in vault files. They are answered by a query over the semantic index, which records each page's outgoing links.
**Migration**: Nothing to run by hand. `cerbo index` deletes leftover `backrefs.ttl` files (see "Remove legacy backrefs files"). Backlinks come from `cerbo page backlinks <uuid>` or the desktop panel.

## ADDED Requirements

### Requirement: Rebuild the semantic graph index

`cerbo index` SHALL rebuild the semantic graph index after regenerating annotations, using the same scope: all vaults, `--vault`, or `--page`. With `--page`, only that page's triples SHALL be replaced in the index.

#### Scenario: Default run rebuilds the graph
- **WHEN** the user runs `cerbo index`
- **THEN** every vault's semantic index SHALL match its files afterwards

#### Scenario: Single page
- **WHEN** the user runs `cerbo index --page <uuid>`
- **THEN** only that page's triples SHALL be replaced in the index

### Requirement: Remove legacy backrefs files

A full `cerbo index` of a vault SHALL delete every `backrefs.ttl` file of live objects in that vault and report how many it deleted. Cerbo SHALL ignore `backrefs.ttl` files everywhere else, and SHALL NOT read backlinks from them.

#### Scenario: Upgrading an existing vault
- **WHEN** a vault still contains 137 `backrefs.ttl` files and the user runs `cerbo index`
- **THEN** no `backrefs.ttl` SHALL remain under `.cerbo/objects/`
- **THEN** the summary SHALL report 137 removed legacy files

#### Scenario: Leftover file is ignored
- **WHEN** an object still has a `backrefs.ttl` that names a page which no longer links to it
- **THEN** that page SHALL NOT be listed as a backlink

### Requirement: Refresh embeddings on request

`cerbo index --embeddings` SHALL compute embeddings for every page in scope whose content changed since its last embedding, then refresh the `cerbo:similarTo` links. Unchanged pages SHALL NOT be re-embedded.

#### Scenario: Only changed pages are embedded
- **WHEN** the user runs `cerbo index --embeddings` twice and edits one page in between
- **THEN** the second run SHALL compute exactly one embedding

### Requirement: Summary includes semantic index statistics

The completion summary of `cerbo index` SHALL include the number of triples in the index, the number of legacy `backrefs.ttl` files removed and, when `--embeddings` is given, the number of pages embedded. With `--json`, these SHALL be the fields `triples`, `legacy_backrefs_removed` and `embedded`.

#### Scenario: JSON summary
- **WHEN** the user runs `cerbo index --embeddings --json`
- **THEN** the JSON summary SHALL contain numeric `triples`, `legacy_backrefs_removed` and `embedded` fields
