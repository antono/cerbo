# Wikilink Editing

## Purpose
Provide a rich editing experience for creating and navigating internal `cerbo://<uuid>` links.

## Requirements

### Requirement: Link format in page.md
The system SHALL use `[Link Text](cerbo://<uuid>)` format for all internal links in `page.md`. The system SHALL NOT use `[[Page Title]]` syntax. Links SHALL NOT include `/page.md` or filename suffix.

#### Scenario: Create link to page
- **WHEN** user creates a link to another page
- **THEN** the markdown SHALL be `[Page Title](cerbo://<uuid>)`
- **THEN** the link SHALL NOT use `[[Page Title]]` syntax
- **THEN** the link SHALL NOT include `/page.md` suffix

#### Scenario: Create link to attachment
- **WHEN** user creates a link to an attachment (image, PDF, etc.)
- **THEN** the markdown SHALL be `![Alt](cerbo://<uuid>)` or `[Text](cerbo://<uuid>)`
- **THEN** the link SHALL NOT include the filename in the URL

### Requirement: Link resolution
The system SHALL resolve `cerbo://<uuid>` links by looking up the UUID in `index.json` or by reading `.cerbo/objects/<uuid>/meta.ttl`.

#### Scenario: Resolve page link
- **WHEN** `page.md` contains `[Rust](cerbo://<uuid>)`
- **THEN** the system reads `.cerbo/objects/<uuid>/meta.ttl`
- **THEN** if type is Page/Source/Ontology, it resolves to `page.md`
- **THEN** if type is Attachment, it resolves to the binary filename

#### Scenario: Invalid UUID in link
- **WHEN** `page.md` contains `cerbo://<non-existent-uuid>`
- **THEN** the system SHALL render the link as broken (e.g., with error styling)
- **THEN** the system SHALL NOT crash

### Requirement: Link syntax highlighting
The system SHALL render `[Text](cerbo://<uuid>)` links distinctly in the editor — resolved links in one style, broken links (invalid UUID) in another.

#### Scenario: Render resolved link
- **WHEN** the editor contains `[Rust Ownership](cerbo://<uuid>)` and the UUID exists
- **THEN** the link is rendered as a clickable inline element with a resolved style

#### Scenario: Render broken link
- **WHEN** the editor contains `[Nonexistent](cerbo://<bad-uuid>)` and no matching object exists
- **THEN** the link is rendered with a broken/unresolved style (e.g., muted or struck)

### Requirement: Link navigation
The system SHALL navigate to the target object when the user clicks a resolved `cerbo://<uuid>` link in the editor or preview.

#### Scenario: Click resolved link
- **WHEN** the user clicks a resolved `[Text](cerbo://<uuid>)` link
- **THEN** the application navigates to the target page

#### Scenario: Click broken link
- **WHEN** the user clicks a link with a non-existent UUID
- **THEN** the application offers to create a new page (and update the link to the new UUID)

### Requirement: Link autocomplete
The system SHALL provide autocomplete suggestions when the user types `[` followed by text in the editor, filtering page titles from `index.json` as the user continues typing. On selection, the editor SHALL insert `[Page Title](cerbo://<uuid>)`.

#### Scenario: Trigger autocomplete
- **WHEN** the user begins typing a link in the editor
- **THEN** a dropdown appears listing existing pages in the active vault (from `index.json` `title_to_uuid`)

#### Scenario: Filter autocomplete results
- **WHEN** the user types text matching "rust"
- **THEN** the dropdown filters to pages whose title contains "rust"

#### Scenario: Select autocomplete suggestion
- **WHEN** the user selects a suggestion from the dropdown
- **THEN** the editor inserts `[Page Title](cerbo://<uuid>)` and closes the dropdown
