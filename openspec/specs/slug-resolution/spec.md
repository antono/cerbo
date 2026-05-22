# Slug Resolution

## Purpose
Define title-to-UUID and UUID-to-path resolution. Slugs are eliminated; UUID v4 is the only identifier.

## Requirements

### Requirement: Resolve wikilink to page
The system SHALL resolve a `[[Title]]` wikilink to a page by looking up the title in the vault's `index.json` `title_to_uuid` map. Resolution SHALL be case-insensitive.

#### Scenario: Exact title match
- **WHEN** the wikilink text is "Rust Ownership" and `index.json` maps it to `<uuid>`
- **THEN** the link resolves to `.cerbo/objects/<uuid>/page.md`

#### Scenario: Lowercase link text
- **WHEN** the wikilink text is "rust ownership" (case-insensitive match for "Rust Ownership")
- **THEN** the link resolves to `.cerbo/objects/<uuid>/page.md`

#### Scenario: No matching page
- **WHEN** the wikilink text does not match any title in `index.json`
- **THEN** the link is marked as broken and the user is offered the option to create the page

### Requirement: Title-to-UUID resolution
The system SHALL resolve page titles to UUIDs using `.cerbo/index.json` `title_to_uuid` map. The system SHALL resolve UUIDs to paths using `uuid_to_path` map.

#### Scenario: Resolve title to UUID
- **WHEN** user searches for page with title "My Page"
- **THEN** the system looks up "My Page" in `index.json` `title_to_uuid`
- **THEN** it returns the corresponding UUID

#### Scenario: Resolve UUID to path
- **WHEN** the system needs the path for UUID `<uuid-page>`
- **THEN** it looks up `<uuid-page>` in `index.json` `uuid_to_path`
- **THEN** it returns `objects/<uuid-page>/`

### Requirement: Human-readable display
The system SHALL display page titles (from `meta.ttl`) to users, not UUIDs. UUIDs are internal identifiers only.

#### Scenario: Display page in UI
- **WHEN** the UI lists pages
- **THEN** it reads the title from each page's `meta.ttl`
- **THEN** it displays the title to the user
- **THEN** the UUID is used internally, not shown to the user (unless via `cerbo resolve`)
