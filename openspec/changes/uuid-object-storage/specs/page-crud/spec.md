# Page CRUD (Modified)

## Purpose
Enable creating, reading, updating, and deleting pages within a vault using UUID-based object storage.

## MODIFIED Requirements

### Requirement: Create page
The system SHALL create a new page by generating a UUID v4, creating `.cerbo/objects/<uuid>/` directory, and writing `page.md` with the title as H1 heading. The system SHALL write `meta.ttl` with `type: :Page` (or `:Product`). The UI SHALL provide a focused modal dialog for this operation.

#### Scenario: Create page with valid title
- **WHEN** the user creates a page with title "Rust Ownership" via the New Page dialog
- **THEN** a UUID v4 is generated (e.g., `<uuid-page>`)
- **THEN** directory `.cerbo/objects/<uuid-page>/` is created
- **THEN** `page.md` is created with content `# Rust Ownership`
- **THEN** `meta.ttl` is created with `type: :Page` and `:title "Rust Ownership"`
- **THEN** the system SHALL switch the editor to "Write" mode for the new page
- **THEN** `index.json` is updated with title→UUID and UUID→path mappings

#### Scenario: Create page with existing title
- **WHEN** the user creates a page whose title matches an existing page's title
- **THEN** a new UUID is still generated (titles are not unique identifiers)
- **THEN** both pages exist with different UUIDs
- **THEN** `index.json` maps both titles to their respective UUIDs

### Requirement: Read page
The system SHALL read the raw markdown content of a page from `.cerbo/objects/<uuid>/page.md` using the UUID identifier. The frontend SHALL render the page title from the first level-1 heading (`# `) within the content in the preview mode.

#### Scenario: Read existing page
- **WHEN** the frontend requests the content of a page by UUID
- **THEN** the system returns the raw markdown string from `.cerbo/objects/<uuid>/page.md`
- **THEN** the frontend renders the markdown preview with the title included as the first heading

#### Scenario: Read non-existent page
- **WHEN** the frontend requests a UUID that does not exist
- **THEN** the system SHALL return an error indicating the page does not exist

### Requirement: Write page
The system SHALL write updated markdown content to a page's `page.md` file atomically. The system SHALL extract `cerbo://<uuid>` links to `relations.ttl` and HackMD annotations to `annotations.ttl`. The system SHALL NOT write to `type: :Source` pages (read-only).

#### Scenario: Save page content
- **WHEN** the frontend provides a UUID and updated markdown content
- **THEN** the system writes the content to `.cerbo/objects/<uuid>/page.md`
- **THEN** outgoing `cerbo://` links are extracted to `relations.ttl`
- **THEN** HackMD `[Text]{prefix:Type}` annotations are extracted to `annotations.ttl`
- **THEN** `meta.ttl` `schema:dateModified` is updated

#### Scenario: Write to source type (read-only)
- **WHEN** the user attempts to write to a page with `type: :Source` in `meta.ttl`
- **THEN** the system SHALL return an error "Cannot write to source type (read-only)"
- **THEN** no changes are written to disk

### Requirement: Delete page
The system SHALL delete a page by removing its entire `.cerbo/objects/<uuid>/` directory. The system SHALL NOT delete `type: :Source` pages. This operation SHALL be irreversible and MUST require confirmation via a modal dialog.

#### Scenario: Delete existing page
- **WHEN** the user triggers a delete operation for a page with `type: :Page`
- **THEN** the system SHALL display a confirmation modal showing the page title
- **WHEN** the user confirms the deletion
- **THEN** the directory `.cerbo/objects/<uuid>/` and all its contents are removed
- **THEN** `index.json` is updated to remove the UUID and title mappings
- **THEN** other objects' `relations.ttl` are updated to remove backlinks

#### Scenario: Delete source type (read-only)
- **WHEN** the user attempts to delete a page with `type: :Source`
- **THEN** the system SHALL return an error "Cannot delete source type (read-only)"
- **THEN** no files or folders are removed

### Requirement: List pages
The system SHALL return a list of all pages by scanning `.cerbo/objects/` directories that contain `page.md` and have `type: :Page` or `type: :Source` in `meta.ttl`.

#### Scenario: List pages in a vault
- **WHEN** the frontend requests the page list
- **THEN** the system returns one entry per object with `page.md` and valid page type
- **THEN** each entry includes UUID, title (from `meta.ttl`), and type
- **THEN** objects without `page.md` (attachments, ontologies without page) are excluded

### Requirement: Import source page
The system SHALL import a URL as a new page with `type: :Source` (read-only). The system SHALL fetch the URL content, convert to markdown if needed, and store in `.cerbo/objects/<uuid>/page.md`.

#### Scenario: Import page from URL
- **WHEN** user runs `cerbo import https://example.com/page`
- **THEN** a new UUID is generated
- **THEN** content is fetched and stored in `.cerbo/objects/<uuid>/page.md`
- **THEN** `meta.ttl` is created with `type: :Source` and `:original-url`
- **THEN** the page is read-only (cannot be written or deleted)

## ADDED Requirements

### Requirement: Resolve UUID to path
The system SHALL provide `cerbo resolve <uuid>` command that returns the local filesystem path to the object's primary file based on its type.

#### Scenario: Resolve page UUID
- **WHEN** user runs `cerbo resolve <uuid-page>`
- **THEN** the system returns `/path/to/vault/.cerbo/objects/<uuid-page>/page.md`

#### Scenario: Resolve attachment UUID
- **WHEN** user runs `cerbo resolve <uuid-attachment>`
- **THEN** the system detects the binary filename from the attachment directory
- **THEN** returns `/path/to/vault/.cerbo/objects/<uuid-attachment>/filename`
