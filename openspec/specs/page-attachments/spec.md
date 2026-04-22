# Page Attachments

## Purpose
Enable users to manage file attachments for pages.

## Requirements

### Requirement: Storage layout for attachments
The system SHALL store files associated with a page in an `assets/` subfolder within the page's directory (e.g., `<vault>/<slug>/assets/`).

#### Scenario: First attachment upload
- **WHEN** the user uploads a file to a page that has no attachments
- **THEN** the system SHALL create the `assets/` folder if it does not exist
- **THEN** the file SHALL be saved into that folder

### Requirement: List page attachments
The system SHALL return a list of all files present in a page's `assets/` folder.

#### Scenario: Requesting attachment list
- **WHEN** the user opens the attachments panel for a page
- **THEN** the system SHALL scan the `<slug>/assets/` folder
- **THEN** it returns a list of filenames found within

### Requirement: Add attachment via CLI
The system SHALL provide a CLI command to add a local file as an attachment to a specific page.

#### Scenario: Adding attachment from terminal
- **WHEN** the user runs `cerbo page attachment add <vault-id> <slug> <path-to-file>`
- **THEN** the file is copied to the page's `assets/` folder
- **THEN** the operation returns the final filename

### Requirement: Delete page attachment
The system SHALL allow users to delete an attachment from a page's `assets/` folder.

#### Scenario: Deleting an attachment
- **WHEN** the user clicks "Delete" on an attachment in the sidebar
- **THEN** the system SHALL remove the corresponding file from disk
- **THEN** the attachment list is updated
