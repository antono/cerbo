# Slug Resolution (Modified)

## Purpose
Remove or replace slug concept (slug eliminated, UUID is the only identifier).

## REMOVED Requirements

### Requirement: Slug derivation from title
**Reason**: Slugs are eliminated in UUID-based storage model. UUID v4 is the only identifier.
**Migration**: Title is stored in `meta.ttl` and can be changed without affecting the UUID.

### Requirement: Slug-based file paths
**Reason**: Files are stored under `.cerbo/objects/<uuid>/`, not `<slug>/`.
**Migration**: All file path computations must use UUID instead of slug.

### Requirement: Title-to-slug resolution
**Reason**: No slugs exist. Title-to-UUID resolution is done via `index.json`.
**Migration**: Use `cerbo resolve <uuid>` or lookup in `index.json`.

## ADDED Requirements

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
