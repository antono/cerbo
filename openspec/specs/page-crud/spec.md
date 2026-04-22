# Page CRUD

## Purpose
Enable creating, reading, updating, and deleting pages within a vault.

## Requirements

### Requirement: Create page
The system SHALL create a new page by deriving a slug from the provided title, creating a folder named after the slug inside the active vault, and writing an empty `page.md` with the title as an H1 heading.

#### Scenario: Create page with valid title
- **WHEN** the user creates a page with title "Rust Ownership"
- **THEN** the folder `rust-ownership/` is created in the vault root
- **THEN** `rust-ownership/page.md` is created with content `# Rust Ownership`

#### Scenario: Create page with title that conflicts with existing slug
- **WHEN** the user creates a page whose derived slug matches an existing page's slug
- **THEN** the system SHALL reject the operation with a descriptive error
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
The system SHALL delete a page by removing its entire folder (including all assets) from the vault. This operation SHALL be irreversible.

#### Scenario: Delete existing page
- **WHEN** the user deletes a page
- **THEN** the page folder and all its contents are removed from disk
- **THEN** the link index is updated to remove the page and its outbound links

### Requirement: List pages
The system SHALL return a list of all pages in the active vault by scanning for folders containing a `page.md` file.

#### Scenario: List pages in a populated vault
- **WHEN** the frontend requests the page list
- **THEN** the system returns one entry per folder containing `page.md`, including slug and title
- **THEN** folders without `page.md` are excluded
