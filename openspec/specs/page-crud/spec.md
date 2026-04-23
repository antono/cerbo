# Page CRUD

## Purpose
Enable creating, reading, updating, and deleting pages within a vault.

## Requirements

### Requirement: Create page
The system SHALL create a new page by deriving a slug from the provided title, creating a folder named after the slug inside the active vault, and writing an empty `page.md` with the title as an H1 heading. The UI SHALL provide a focused modal dialog with a live slug preview for this operation.

#### Scenario: Create page with valid title
- **WHEN** the user creates a page with title "Rust Ownership" via the New Page dialog
- **THEN** the folder `rust-ownership/` is created in the vault root
- **THEN** `rust-ownership/page.md` is created with content `# Rust Ownership`
- **THEN** the system SHALL switch the editor to "Write" mode for the new page

#### Scenario: Create page with title that conflicts with existing slug
- **WHEN** the user creates a page whose derived slug matches an existing page's slug
- **THEN** the system SHALL reject the operation with a descriptive error in the dialog
- **THEN** no folder is created

### Requirement: Read page
The system SHALL read the raw markdown content of a page from its `page.md` file and return it to the frontend. The frontend SHALL render the page title from the first level-1 heading (`# `) within the content in the preview mode.

#### Scenario: Read existing page
- **WHEN** the frontend requests the content of a page by slug
- **THEN** the system returns the raw markdown string from `<slug>/page.md`
- **THEN** the frontend renders the markdown preview with the title included as the first heading

#### Scenario: Read non-existent page
- **WHEN** the frontend requests a slug that has no corresponding folder
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
The system SHALL write updated markdown content to a page's `page.md` file atomically.

#### Scenario: Save page content
- **WHEN** the frontend provides a slug and updated markdown content
- **THEN** the system writes the content to `<slug>/page.md`
- **THEN** the FS watcher triggers an incremental link index update

### Requirement: Delete page
The system SHALL delete a page by removing its entire folder (including all assets) from the vault. This operation SHALL be irreversible and MUST require confirmation via a modal dialog.

#### Scenario: Delete existing page
- **WHEN** the user triggers a delete operation
- **THEN** the system SHALL display a confirmation modal showing the page title and slug (e.g., `Title [slug:slug]`)
- **WHEN** the user confirms the deletion
- **THEN** the page folder and all its contents are removed from disk
- **THEN** the link index is updated to remove the page and its outbound links

### Requirement: Rename page
The system SHALL rename a page by updating its title (H1 heading in `page.md`) and/or its slug (folder name). The UI SHALL provide a focused modal dialog showing current metadata and a preview of the new slug.

#### Scenario: Rename page title and slug
- **WHEN** the user provides a new title "Advanced Rust" for a page with slug "rust-intro"
- **THEN** the folder is renamed from `rust-intro/` to `advanced-rust/`
- **THEN** the H1 heading in `page.md` is updated to `# Advanced Rust`
- **THEN** the link index is updated to reflect the new slug

#### Scenario: Rename results in slug conflict
- **WHEN** the user renames a page to a title whose derived slug already exists
- **THEN** the system SHALL reject the operation with a descriptive error in the dialog
- **THEN** no files or folders are moved

### Requirement: Bidirectional Title Sync
The system SHALL maintain synchronization between the page's metadata (title/slug) and the first H1 heading in its markdown content.

#### Scenario: Update title via markdown
- **WHEN** the user edits the first `# Heading` in the markdown editor
- **THEN** after the auto-save delay, the system SHALL trigger a page rename
- **THEN** the folder SHALL be renamed to match the new derived slug
- **THEN** the sidebar and internal state SHALL update to reflect the new title and slug

#### Scenario: Update markdown via rename dialog
- **WHEN** the user renames a page via the modal dialog
- **THEN** the system SHALL update the first `# Heading` in the `page.md` file to match the new title
- **THEN** the editor SHALL reload the updated content to show the new heading

### Requirement: Mandatory Title in Markdown
The system SHALL ensure that every page has an H1 heading in its markdown content. If a title is missing after an edit or save operation, the system SHALL automatically infer one.

#### Scenario: Save page without H1 heading
- **WHEN** the user saves a page (via editor auto-save or CLI) that lacks an H1 heading (`# Title`)
- **THEN** the system SHALL infer a title from the current page metadata or the slug (e.g., humanizing `my-long-slug` to `My Long Slug`)
- **THEN** the system SHALL prepend the inferred title as an H1 heading to the markdown content before writing to disk

### Requirement: List pages
The system SHALL return a list of all pages in the active vault by scanning for folders containing a `page.md` file.

#### Scenario: List pages in a populated vault
- **WHEN** the frontend requests the page list
- **THEN** the system returns one entry per folder containing `page.md`, including slug and title
- **THEN** folders without `page.md` are excluded
