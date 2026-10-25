# Page CRUD

## MODIFIED Requirements

### Requirement: Create page
The system SHALL create a new page by generating a UUID v4, creating `.cerbo/objects/<uuid>/`
directory, and writing `page.md` with the title as H1 heading. The system SHALL write
`meta.ttl` with `type: :Page` (or `:Product`). The system SHALL automatically populate
`cerbo:slug` in `meta.ttl` by deriving it from the title using the project slug algorithm
(kebab-case ASCII, deunicode-transliterated, lowercase, 1..=80 characters, fallback to
`untitled-<first-8-of-uuid>` for empty results). Callers MAY optionally supply an initial
`cerbo:virtualPath`; if omitted, the predicate SHALL NOT be written (equivalent to placing
the page at the symlink-tree root). The UI SHALL provide a focused modal dialog for this
operation. Page creation SHALL NOT write to any vault-wide index file, and its cost SHALL NOT
grow with the number of pages already in the vault.

#### Scenario: Create page with valid title
- **WHEN** the user creates a page with title "Rust Ownership" via the New Page dialog
- **THEN** a UUID v4 is generated (e.g., `<uuid-page>`)
- **THEN** directory `.cerbo/objects/<uuid-page>/` is created
- **THEN** `page.md` is created with content `# Rust Ownership`
- **THEN** `meta.ttl` is created with `type: :Page`, `:title "Rust Ownership"`, and `cerbo:slug "rust-ownership"`
- **THEN** the system SHALL switch the editor to "Write" mode for the new page
- **THEN** no vault-wide index file SHALL be written

#### Scenario: Create page with existing title
- **WHEN** the user creates a page whose title matches an existing page's title
- **THEN** a new UUID is still generated (titles are not unique identifiers)
- **THEN** both pages exist with different UUIDs
- **THEN** both pages SHALL remain individually resolvable by UUID
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

#### Scenario: Create page in a large vault
- **WHEN** the user creates a page in a vault that already contains 137 pages
- **THEN** the operation SHALL write only files inside the new object's own directory
- **THEN** no existing object's files SHALL be rewritten

### Requirement: Delete page
The system SHALL delete a page by relocating its entire `.cerbo/objects/<uuid>/` directory
into the vault's trash area, as specified by the `object-trash` capability. The system SHALL
NOT delete `type: :Source` pages. The operation MUST require confirmation via a modal dialog.
The page's files SHALL be recoverable from the trash; the operation SHALL NOT destroy content.

#### Scenario: Delete existing page
- **WHEN** the user triggers a delete operation for a page with `type: :Page`
- **THEN** the system SHALL display a confirmation modal showing the page title
- **WHEN** the user confirms the deletion
- **THEN** the directory `.cerbo/objects/<uuid>/` SHALL no longer exist
- **THEN** the page's files SHALL exist under `.cerbo/trash/` with their content preserved
- **THEN** other objects' `backrefs.ttl` are updated to remove backlinks
- **THEN** no vault-wide index file SHALL be written

#### Scenario: Delete source type (read-only)
- **WHEN** the user attempts to delete a page with `type: :Source`
- **THEN** the system SHALL return an error "Cannot delete source type (read-only)"
- **THEN** no files or folders are removed or relocated

#### Scenario: Deleted page is not resurrected by a later save
- **WHEN** a page has been deleted and another page still linking to it is saved
- **THEN** the deleted object's directory SHALL NOT be recreated under `.cerbo/objects/`

### Requirement: Bidirectional Title Sync
The system SHALL maintain synchronization between the page's metadata (title in `meta.ttl`)
and the first H1 heading in its markdown content. `meta.ttl` SHALL be the authoritative
source of a page's title for all listings and lookups; the H1 SHALL NOT be scraped as a
substitute.

#### Scenario: Update title via markdown
- **WHEN** the user edits the first `# Heading` in the markdown editor
- **THEN** after the auto-save delay, the system SHALL update `meta.ttl` `:title` to match
- **THEN** the sidebar and internal state SHALL update to reflect the new title
- **THEN** no vault-wide index file SHALL be written

#### Scenario: Update markdown via rename dialog
- **WHEN** the user renames a page via the modal dialog
- **THEN** the system SHALL update the first `# Heading` in the `page.md` file to match the new title

#### Scenario: Page listing reads titles from metadata
- **WHEN** the system lists the pages in a vault
- **THEN** each page's title SHALL be read from its `meta.ttl`
- **THEN** `page.md` SHALL NOT be read in order to determine a title

#### Scenario: Page whose body H1 disagrees with its metadata
- **WHEN** a page's `page.md` H1 was changed outside Cerbo and no longer matches `meta.ttl` `:title`
- **THEN** page listings SHALL show the title recorded in `meta.ttl`

#### Scenario: Title containing Turtle-significant characters
- **WHEN** the user renames a page to `He said "hi"` via the rename dialog
- **THEN** `meta.ttl` SHALL remain a valid Turtle document
- **THEN** subsequent listings SHALL show exactly `He said "hi"`
