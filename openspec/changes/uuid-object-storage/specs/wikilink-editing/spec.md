# Wikilink Editing (Modified)

## Purpose
Replace `[[Page Title]]` wikilinks with `cerbo://<uuid>` markdown links.

## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: Wikilink syntax [[Title]]
**Reason**: Replaced by `cerbo://<uuid>` markdown links for consistent UUID-based identification.
**Migration**: Existing `[[Title]]` syntax is no longer supported. Users must use `cerbo://<uuid>` format.

### Requirement: Title-to-slug resolution
**Reason**: Slugs are eliminated; UUIDs are the only identifier.
**Migration**: Title-to-UUID resolution is done via `index.json`.
