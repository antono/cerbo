# Page CRUD

## Purpose
Enable creating, reading, updating, and deleting pages within a vault using UUID-based object storage.

## Requirements

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

### Requirement: Unified Editor Interface
The system SHALL provide a unified editor interface using a tabbed layout (e.g., "Write" and "Preview") to switch between editing and viewing modes. The editor SHALL support a standard set of plugins for enhanced productivity.

#### Scenario: Switching to preview mode
- **WHEN** the user selects the "Preview" tab in the editor
- **THEN** the system SHALL render the markdown content as HTML, including the page title
- **THEN** the system SHALL attach interactive handlers to wikilinks within the preview

#### Scenario: Using enhanced editor features
- **WHEN** the user types markdown in the "Write" tab
- **THEN** the system SHALL provide syntax highlighting for code blocks
- **THEN** the system SHALL support emoji shortcodes and a file attachment picker

#### Scenario: Keyboard-driven mode switching
- **WHEN** the user is in preview mode and presses `i` or `Ctrl+I` (Linux/Windows) or `Cmd+I` (Mac)
- **THEN** the system SHALL switch to the "Write" (edit) mode and focus the editor
- **WHEN** the user is in edit mode and presses `Esc`
- **THEN** the system SHALL switch to the "Preview" mode

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

### Requirement: Rename page
The system SHALL rename a page by updating its title (H1 heading in `page.md`) and its `:title` in `meta.ttl`. The page's UUID and directory location SHALL NOT change. The UI SHALL provide a focused modal dialog showing current metadata.

#### Scenario: Rename page title
- **WHEN** the user provides a new title "Advanced Rust" for a page
- **THEN** the H1 heading in `page.md` is updated to `# Advanced Rust`
- **THEN** the `:title` in `meta.ttl` is updated to "Advanced Rust"
- **THEN** `index.json` `title_to_uuid` is updated to map "Advanced Rust" to the page's UUID
- **THEN** the UUID and directory location SHALL NOT change

### Requirement: Bidirectional Title Sync
The system SHALL maintain synchronization between the page's metadata (title in `meta.ttl`) and the first H1 heading in its markdown content.

#### Scenario: Update title via markdown
- **WHEN** the user edits the first `# Heading` in the markdown editor
- **THEN** after the auto-save delay, the system SHALL update `meta.ttl` `:title` to match
- **THEN** `index.json` `title_to_uuid` SHALL update to reflect the new title
- **THEN** the sidebar and internal state SHALL update to reflect the new title

#### Scenario: Update markdown via rename dialog
- **WHEN** the user renames a page via the modal dialog
- **THEN** the system SHALL update the first `# Heading` in the `page.md` file to match the new title
- **THEN** the editor SHALL reload the updated content to show the new heading

### Requirement: Mandatory Title in Markdown
The system SHALL ensure that every page has an H1 heading in its markdown content. If a title is missing after an edit or save operation, the system SHALL automatically infer one.

#### Scenario: Save page without H1 heading
- **WHEN** the user saves a page (via editor auto-save or CLI) that lacks an H1 heading (`# Title`)
- **THEN** the system SHALL infer a title from the current page metadata (`meta.ttl` `:title`)
- **THEN** the system SHALL prepend the inferred title as an H1 heading to the markdown content before writing to disk

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

### Requirement: Resolve UUID to path
The system SHALL provide `cerbo resolve <uuid>` command that returns the local filesystem path to the object's primary file based on its type.

#### Scenario: Resolve page UUID
- **WHEN** user runs `cerbo resolve <uuid-page>`
- **THEN** the system returns `/path/to/vault/.cerbo/objects/<uuid-page>/page.md`

#### Scenario: Resolve attachment UUID
- **WHEN** user runs `cerbo resolve <uuid-attachment>`
- **THEN** the system detects the binary filename from the attachment directory
- **THEN** returns `/path/to/vault/.cerbo/objects/<uuid-attachment>/filename`
