## ADDED Requirements

### Requirement: On-demand filesystem conflict diff preview
When a page file changes on disk while the page is open, the system SHALL let the user request a diff preview before resolving the conflict.

#### Scenario: Open conflict prompt
- **WHEN** the filesystem watcher reports a change for the current page
- **THEN** the system SHALL show the existing conflict prompt with `Load changes` and `Overwrite` actions
- **THEN** the system SHALL NOT show the diff preview until the user requests it

#### Scenario: Request diff preview
- **WHEN** the user selects the preview action from the conflict prompt
- **THEN** the system SHALL re-read the current file content from disk
- **THEN** the system SHALL compare the disk content against the in-memory editor content
- **THEN** the system SHALL display the comparison in an expanded diff preview view

#### Scenario: Preview view presentation
- **WHEN** the expanded diff preview is shown
- **THEN** the system SHALL keep the conflict header compact with the icon and title on one row
- **THEN** the system SHALL render an opaque modal surface above the rest of the app chrome
- **THEN** the system SHALL NOT show a redundant refresh control in the preview view

### Requirement: Approve conflict from diff preview
The diff preview view SHALL allow the user to resolve the conflict without returning to the prompt.

#### Scenario: Approve disk changes from diff view
- **WHEN** the diff preview is open and the user selects `Load changes`
- **THEN** the system SHALL load the disk content into the editor state
- **THEN** the system SHALL close the conflict UI

#### Scenario: Keep editor content from diff view
- **WHEN** the diff preview is open and the user selects `Overwrite`
- **THEN** the system SHALL write the in-memory editor content to disk
- **THEN** the system SHALL close the conflict UI

### Requirement: Diff preview uses live disk content
The system SHALL generate the diff preview from the current file contents on disk and the current editor buffer at the moment the user requests preview.

#### Scenario: Disk changes after prompt opens
- **WHEN** the file changes again after the conflict prompt is already open
- **AND WHEN** the user later requests the diff preview
- **THEN** the system SHALL use the latest file contents from disk for the preview
