# Slug Resolution

## MODIFIED Requirements

### Requirement: Resolve wikilink to page
The system SHALL resolve a `[[Title]]` wikilink to a page by matching the title against the
titles recorded in objects' `meta.ttl`. Resolution SHALL be case-insensitive. The system
SHALL NOT depend on any single on-disk index file for this lookup; the title set SHALL be
derivable entirely from the objects in the vault.

#### Scenario: Exact title match
- **WHEN** the wikilink text is "Rust Ownership" and an object's `meta.ttl` records that title
- **THEN** the link resolves to `.cerbo/objects/<uuid>/page.md`

#### Scenario: Lowercase link text
- **WHEN** the wikilink text is "rust ownership" (case-insensitive match for "Rust Ownership")
- **THEN** the link resolves to `.cerbo/objects/<uuid>/page.md`

#### Scenario: No matching page
- **WHEN** the wikilink text does not match the title of any object in the vault
- **THEN** the link is marked as broken and the user is offered the option to create the page

#### Scenario: Resolution survives a missing or stale cache
- **WHEN** any derived title lookup is absent, empty, or out of date
- **THEN** resolution SHALL still return the correct result by consulting the objects themselves

### Requirement: Title-to-UUID resolution
The system SHALL resolve page titles to UUIDs from the titles recorded in objects'
`meta.ttl`. An object's storage location SHALL be derived from its UUID rather than looked up
in a stored map. Where a lookup structure is held for performance, it SHALL be a cache that
can be rebuilt from the objects at any time, and SHALL never be the authority.

#### Scenario: Resolve title to UUID
- **WHEN** the user searches for a page with title "My Page"
- **THEN** the system SHALL find the object whose `meta.ttl` records the title "My Page"
- **THEN** it SHALL return that object's UUID

#### Scenario: Resolve UUID to path
- **WHEN** the system needs the path for UUID `<uuid-page>`
- **THEN** it SHALL derive the path `objects/<uuid-page>/` from the UUID
- **THEN** no stored mapping SHALL be consulted

#### Scenario: Duplicate titles
- **WHEN** two objects record the same title in their `meta.ttl`
- **THEN** both objects SHALL remain individually resolvable by UUID
- **THEN** title-based resolution SHALL report the ambiguity rather than silently discarding one object

#### Scenario: Newly created page is immediately resolvable
- **WHEN** a page is created and then resolved by title in the same session
- **THEN** resolution SHALL succeed without requiring a separate index update step
