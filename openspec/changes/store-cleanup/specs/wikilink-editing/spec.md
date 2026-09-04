# Wikilink Editing

## MODIFIED Requirements

### Requirement: Link resolution
The system SHALL resolve `cerbo://<uuid>` links by reading `.cerbo/objects/<uuid>/meta.ttl`.
An object SHALL be considered to exist only if it has readable metadata; the presence of a
directory alone SHALL NOT constitute existence.

#### Scenario: Resolve page link
- **WHEN** `page.md` contains `[Rust](cerbo://<uuid>)`
- **THEN** the system reads `.cerbo/objects/<uuid>/meta.ttl`
- **THEN** if type is Page/Source/Ontology, it resolves to `page.md`
- **THEN** if type is Attachment, it resolves to the binary filename

#### Scenario: Resolve link to a non-existent UUID
- **WHEN** the user clicks a link with a non-existent UUID
- **THEN** the application offers to create a new page (and update the link to the new UUID)

#### Scenario: Directory without metadata does not resolve
- **WHEN** `.cerbo/objects/<uuid>/` exists but contains no `meta.ttl`
- **THEN** resolution SHALL report the object as not found
- **THEN** the object SHALL NOT be treated as writable

#### Scenario: Invalid UUID in link
- **WHEN** `page.md` contains `cerbo://<non-existent-uuid>`
- **THEN** the system SHALL render the link as broken (e.g., with error styling)
- **THEN** the system SHALL NOT crash

#### Scenario: Malformed UUID is never used as a path
- **WHEN** a link's UUID is not a syntactically valid UUID
- **THEN** resolution SHALL fail with a broken-link result
- **THEN** the value SHALL NOT be used to construct a filesystem path

### Requirement: Link autocomplete
The system SHALL provide autocomplete suggestions when the user types `[` followed by text in
the editor, filtering page titles drawn from the vault's objects as the user continues
typing. On selection, the editor SHALL insert `[Page Title](cerbo://<uuid>)`.

#### Scenario: Trigger autocomplete
- **WHEN** the user begins typing a link in the editor
- **THEN** a dropdown appears listing existing pages in the active vault, titled from their `meta.ttl`

#### Scenario: Filter autocomplete results
- **WHEN** the user types text matching "rust"
- **THEN** the dropdown filters to pages whose title contains "rust"

#### Scenario: Select autocomplete suggestion
- **WHEN** the user selects a suggestion from the dropdown
- **THEN** the editor inserts `[Page Title](cerbo://<uuid>)` and closes the dropdown

#### Scenario: Newly created page appears in autocomplete
- **WHEN** a page is created and autocomplete is triggered afterwards in the same session
- **THEN** the new page SHALL appear among the suggestions

#### Scenario: Trashed page does not appear in autocomplete
- **WHEN** a page has been deleted and autocomplete is triggered
- **THEN** the deleted page SHALL NOT appear among the suggestions
