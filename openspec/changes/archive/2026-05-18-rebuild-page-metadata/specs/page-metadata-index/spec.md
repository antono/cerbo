## ADDED Requirements

### Requirement: Index page metadata with scope control

The system SHALL provide a CLI command `cerbo index` that indexes `backrefs.ttl` and `annotations.ttl` files by re-parsing page content, with support for three scopes: all pages (default), single vault, or single page.

#### Scenario: Index all pages across all vaults
- **WHEN** user runs `cerbo index` with no arguments
- **THEN** system scans all vaults, processes all pages, and regenerates backrefs and annotations for each page

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

### Requirement: Extract and regenerate backref metadata

The system SHALL parse each `page.md` file for `cerbo://<uuid>` links and update the corresponding `backrefs.ttl` files for each referenced page.

#### Scenario: Page with outgoing links
- **WHEN** page A contains `cerbo://uuid-b` and `cerbo://uuid-c`
- **THEN** system adds page A's UUID to `backrefs.ttl` in both page B and page C's object directories

#### Scenario: Page with no links
- **WHEN** page contains no `cerbo://` links
- **THEN** no backref files are modified for other pages (but this page's backrefs may still be updated by other pages linking to it)

#### Scenario: Broken link reference
- **WHEN** page contains `cerbo://nonexistent-uuid`
- **THEN** system logs a warning but continues processing (orphaned link)

### Requirement: Extract and regenerate annotation metadata

The system SHALL parse each `page.md` file for `[text]{prefix:Type}` annotations and regenerate the `annotations.ttl` file for that page.

#### Scenario: Page with annotations
- **WHEN** page contains `[John Doe]{foaf:Person}` and `[ACME Corp]{schema:Organization}`
- **THEN** system writes `annotations.ttl` with both annotations, including line/column positions

#### Scenario: Page with no annotations
- **WHEN** page contains no annotation syntax
- **THEN** `annotations.ttl` is either removed or written as empty (depending on implementation)

### Requirement: Provide progress feedback

The system SHALL display progress information during indexing operations for user feedback.

#### Scenario: Progress updates
- **WHEN** indexing a vault with 500 pages
- **THEN** system displays periodic updates like "Processing 100/500 pages..." to stderr

#### Scenario: Summary on completion
- **WHEN** indexing completes
- **THEN** system outputs summary statistics to stdout: total pages processed, links found, annotations extracted, errors encountered

#### Scenario: Error reporting
- **WHEN** errors occur during indexing
- **THEN** system logs error details (file path, error message) to stderr and exits with non-zero code

### Requirement: Idempotent operation

The system SHALL produce identical results when run multiple times on the same vault state.

#### Scenario: Multiple runs on all pages
- **WHEN** user runs `cerbo index` twice on unchanged vault
- **THEN** second run produces identical backrefs.ttl and annotations.ttl files as first run

#### Scenario: Multiple runs on single page
- **WHEN** user runs `cerbo index --page <uuid>` twice without changing the page
- **THEN** second run produces identical backrefs.ttl and annotations.ttl as first run

#### Scenario: Incremental changes
- **WHEN** user adds a link to page A, runs index, then removes the link and runs index again
- **THEN** final state matches original state (backref removed from target page)
