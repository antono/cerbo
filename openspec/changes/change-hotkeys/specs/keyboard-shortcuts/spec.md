## MODIFIED Requirements

### Requirement: Preview Mode Page Operations
The system MUST provide keyboard shortcuts for common page operations while in "Preview" mode.

#### Scenario: Scroll page in preview mode
- **WHEN** the user is in preview mode and not focusing an input
- **AND** the user presses `j` or `ArrowDown`
- **THEN** the system SHALL scroll the preview viewport down by 100 pixels

#### Scenario: Scroll page up in preview mode
- **WHEN** the user is in preview mode and not focusing an input
- **AND** the user presses `k` or `ArrowUp`
- **THEN** the system SHALL scroll the preview viewport up by 100 pixels

#### Scenario: Navigate to next page with J
- **WHEN** the user is in preview mode and not focusing an input
- **AND** the user presses `J` (uppercase)
- **THEN** the system SHALL open the next page in the sidebar list

#### Scenario: Navigate to previous page with K
- **WHEN** the user is in preview mode and not focusing an input
- **AND** the user presses `K` (uppercase)
- **THEN** the system SHALL open the previous page in the sidebar list

#### Scenario: Trigger rename in preview mode
- **WHEN** the user is in preview mode and not focusing an input
- **AND** the user presses `r`
- **THEN** the system SHALL open the rename dialog for the current page

#### Scenario: Trigger delete in preview mode
- **WHEN** the user is in preview mode and not focusing an input
- **AND** the user presses `Delete` or `Backspace`
- **THEN** the system SHALL open the delete confirmation dialog for the current page

### Requirement: Add Vault Shortcut
The system MUST provide a global keyboard shortcut to trigger the "Add Vault" workflow.

#### Scenario: Trigger Add Vault via Ctrl+Shift+O
- **WHEN** the user presses `Ctrl+Shift+O` (Linux/Windows) or `Cmd+Shift+O` (Mac)
- **THEN** the native system directory picker SHALL be displayed for vault selection to add

### Requirement: Vault Selector Shortcut
The system MUST provide a global keyboard shortcut to open the vault selector.

#### Scenario: Open vault selector via Ctrl+O
- **WHEN** the user presses `Ctrl+O` (Linux/Windows) or `Cmd+O` (Mac)
- **THEN** the vault selector SHALL be displayed

### Requirement: Sidebar Navigation Shortcuts
The system MUST provide keyboard shortcuts for efficient navigation within the sidebar's page list.

#### Scenario: Sidebar shortcuts listed in help
- **WHEN** the Shortcuts Help modal is opened
- **THEN** it SHALL include a section for "Sidebar Navigation"
- **AND** it SHALL list `J` for "Next page", `K` for "Previous page", `j` for "Scroll down", `k` for "Scroll up", and `Tab` for "Cycle pages"
