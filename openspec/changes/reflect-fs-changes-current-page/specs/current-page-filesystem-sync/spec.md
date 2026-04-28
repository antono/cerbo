## ADDED Requirements

### Requirement: Active page refresh on filesystem change
The system SHALL detect when the backing `page.md` file for the currently open page changes on disk and SHALL refresh only that page's displayed content.

#### Scenario: Refresh current page after external edit
- **WHEN** the user has a page open and its `page.md` changes outside the app
- **THEN** the application SHALL update the current page view from the latest file content
- **THEN** the application SHALL NOT navigate to a different page

### Requirement: Preview mode reloads from disk
When the current page is open in preview mode and its backing file changes on disk, the system SHALL re-read the file and re-render the preview from the updated content.

#### Scenario: Re-render preview from changed file
- **WHEN** the current page is in preview mode and `page.md` changes on disk
- **THEN** the system SHALL reload the markdown from disk
- **THEN** the preview SHALL reflect the new file content

### Requirement: Edit mode conflict choice
When the current page is open in edit mode and its backing file changes on disk, the system SHALL prompt the user to choose between loading the external changes or overwriting disk with the in-app content.

#### Scenario: Resolve edit-mode file conflict
- **WHEN** the current page is in edit mode and `page.md` changes on disk
- **THEN** the application SHALL display a dialog with load and overwrite choices
- **THEN** the system SHALL apply only the user's chosen action
