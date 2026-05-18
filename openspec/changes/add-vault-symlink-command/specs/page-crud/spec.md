## MODIFIED Requirements

### Requirement: Create page
The system SHALL create a new page by generating a UUID v4, creating `.cerbo/objects/<uuid>/` directory, and writing `page.md` with the title as H1 heading. The system SHALL write `meta.ttl` with `type: :Page` (or `:Product`). The system SHALL automatically populate `cerbo:slug` in `meta.ttl` by deriving it from the title using the project slug algorithm (kebab-case ASCII, deunicode-transliterated, lowercase, 1..=80 characters, fallback to `untitled-<first-8-of-uuid>` for empty results). Callers MAY optionally supply an initial `cerbo:virtualPath`; if omitted, the predicate SHALL NOT be written (equivalent to placing the page at the symlink-tree root). The UI SHALL provide a focused modal dialog for this operation.

#### Scenario: Create page with valid title
- **WHEN** the user creates a page with title "Rust Ownership" via the New Page dialog
- **THEN** a UUID v4 is generated (e.g., `<uuid-page>`)
- **THEN** directory `.cerbo/objects/<uuid-page>/` is created
- **THEN** `page.md` is created with content `# Rust Ownership`
- **THEN** `meta.ttl` is created with `type: :Page`, `:title "Rust Ownership"`, and `cerbo:slug "rust-ownership"`
- **THEN** the system SHALL switch the editor to "Write" mode for the new page
- **THEN** `index.json` is updated with title→UUID and UUID→path mappings

#### Scenario: Create page with existing title
- **WHEN** the user creates a page whose title matches an existing page's title
- **THEN** a new UUID is still generated (titles are not unique identifiers)
- **THEN** both pages exist with different UUIDs
- **THEN** `index.json` maps both titles to their respective UUIDs
- **THEN** the auto-generated `cerbo:slug` for the new page may collide with the existing page's slug; this collision is surfaced by `cerbo index` and blocks `cerbo symlink` until the user resolves it by editing one of the slugs or virtualPaths

#### Scenario: Create page with explicit virtualPath
- **WHEN** a caller creates a page and supplies `cerbo:virtualPath "notes/rust"`
- **THEN** `meta.ttl` SHALL include `cerbo:virtualPath "notes/rust"` as an independent Turtle triple

#### Scenario: Create page with title that transliterates to empty
- **WHEN** the user creates a page whose title slugifies to an empty string (e.g. emoji-only title)
- **THEN** `cerbo:slug` SHALL be set to `untitled-<first-8-chars-of-uuid>`

#### Scenario: Create page without explicit virtualPath
- **WHEN** the user creates a page without supplying `cerbo:virtualPath`
- **THEN** `meta.ttl` SHALL NOT contain a `cerbo:virtualPath` predicate
