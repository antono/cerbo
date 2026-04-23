# Keyboard Shortcuts

## Purpose
Provide standard and efficient keyboard-driven navigation and control across the application.

## Requirements

### Requirement: Global Page Search Shortcut
The system MUST provide a global keyboard shortcut to open the page autocomplete/search interface from anywhere in the application.

#### Scenario: Open search via Ctrl+P
- **WHEN** the user presses `Ctrl+P` (Linux/Windows) or `Cmd+P` (Mac)
- **THEN** the Page Autocomplete interface SHALL be displayed and focused

### Requirement: Create Page Shortcut
The system MUST provide a global keyboard shortcut to open the "New Page" form from anywhere in the application.

#### Scenario: Open new page form via Ctrl+N
- **WHEN** the user presses `Ctrl+N` (Linux/Windows) or `Cmd+N` (Mac)
- **THEN** the New Page form SHALL be displayed and focused
- **THEN** any other active dialogs or forms (Search, Vault Switcher) SHALL be automatically closed

### Requirement: Escape to Close
The system MUST use the `Esc` key as a universal "close" or "cancel" action for all transient UI elements, including modals, forms, the vault switcher, and delete confirmations.

#### Scenario: Close active form via Esc
- **WHEN** any transient UI element is active (New Page form, Search modal, Vault Switcher, Rename form, Delete confirmation)
- **AND** the user presses `Esc`
- **THEN** the active element SHALL be closed and focus returned to the previous context

### Requirement: Cross-Platform Key Mapping
The hotkey system MUST automatically map `Ctrl` (Linux/Windows) to `Cmd` (Mac) for all application-level shortcuts to ensure a native experience on all supported platforms.

#### Scenario: Mapping shortcuts on Mac
- **WHEN** the application is running on macOS
- **THEN** all defined `Ctrl` shortcuts MUST be triggered by the `Cmd` key instead

### Requirement: Theme Toggle Shortcut
The system MUST provide a global keyboard shortcut to toggle between Light and Dark themes.

#### Scenario: Toggle theme via Ctrl+T
- **WHEN** the user presses `Ctrl+T` (Linux/Windows) or `Cmd+T` (Mac)
- **THEN** the application theme SHALL toggle (Light to Dark, or Dark to Light)

### Requirement: Shortcuts Help Shortcut
The system MUST provide a global keyboard shortcut to open the Shortcuts Help modal.

#### Scenario: Open help via F1
- **WHEN** the user presses `F1`
- **THEN** the Shortcuts Help modal SHALL be displayed

### Requirement: Add Vault Shortcut
The system MUST provide a global keyboard shortcut to trigger the "Add Vault" workflow.

#### Scenario: Trigger Add Vault via Ctrl+O
- **WHEN** the user presses `Ctrl+O` (Linux/Windows) or `Cmd+O` (Mac)
- **THEN** the native system directory picker SHALL be displayed for vault selection

### Requirement: Sidebar Navigation Shortcuts
The system MUST provide keyboard shortcuts for efficient navigation within the sidebar's page list.

#### Scenario: Sidebar shortcuts listed in help
- **WHEN** the Shortcuts Help modal is opened
- **THEN** it SHALL include a section for "Sidebar Navigation"
- **AND** it SHALL list `ArrowUp/k` for "Previous page", `ArrowDown/j` for "Next page", and `Tab` for "Cycle pages"

### Requirement: Preview Mode Page Operations
The system MUST provide keyboard shortcuts for common page operations while in "Preview" mode.

#### Scenario: Navigate pages in preview mode
- **WHEN** the user is in preview mode and not focusing an input
- **AND** the user presses `j` or `ArrowDown`
- **THEN** the system SHALL open the next page in the list
- **WHEN** the user presses `k` or `ArrowUp`
- **THEN** the system SHALL open the previous page in the list

#### Scenario: Trigger rename in preview mode
- **WHEN** the user is in preview mode and not focusing an input
- **AND** the user presses `r`
- **THEN** the system SHALL open the rename dialog for the current page

#### Scenario: Trigger delete in preview mode
- **WHEN** the user is in preview mode and not focusing an input
- **AND** the user presses `Delete` or `Backspace`
- **THEN** the system SHALL open the delete confirmation dialog for the current page
