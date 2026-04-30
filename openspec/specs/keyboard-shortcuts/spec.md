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
The system MUST provide a global keyboard shortcut to open the "New Page" dialog from anywhere in the application.

#### Scenario: Open new page dialog via Ctrl+N
- **WHEN** the user presses `Ctrl+N` (Linux/Windows) or `Cmd+N` (Mac)
- **THEN** the New Page dialog SHALL be displayed as a modal and focused
- **THEN** any other active dialogs or forms (Search, Vault Switcher) SHALL be automatically closed

### Requirement: Escape to Close
The system MUST use the `Esc` key as a universal "close" or "cancel" action for all transient UI elements, including modals (New Page, Search, Help), forms, the vault switcher, and delete confirmations.

#### Scenario: Close active modal via Esc
- **WHEN** any transient UI element is active (New Page dialog, Search modal, Vault Switcher, Rename form, Delete confirmation)
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

#### Scenario: Trigger Add Vault via Ctrl+Shift+O
- **WHEN** the user presses `Ctrl+Shift+O` (Linux/Windows) or `Cmd+Shift+O` (Mac)
- **THEN** the native system directory picker SHALL be displayed for vault selection

### Requirement: Vault Selector Shortcut
The system MUST provide a global keyboard shortcut to open the vault selector.

#### Scenario: Open vault selector via Ctrl+Shift+O
- **WHEN** the user presses `Ctrl+Shift+O` (Linux/Windows) or `Cmd+Shift+O` (Mac)
- **THEN** the vault selector SHALL be displayed

### Requirement: Existing Shortcuts Preserved
The keyboard shortcuts list MUST continue to document the existing page search, new page, theme toggle, help, and quit shortcuts.

### Requirement: Sidebar Navigation Shortcuts
The system MUST provide keyboard shortcuts for efficient navigation within the sidebar's page list.

#### Scenario: Sidebar shortcuts listed in help
- **WHEN** the Shortcuts Help modal is opened
- **THEN** it SHALL include a section for "Sidebar Navigation"
- **AND** it SHALL list `J` for "Next page", `K` for "Previous page", `j` for "Scroll down", `k` for "Scroll up", and `Tab` for "Cycle pages"

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

### Requirement: Go Back Shortcut
The system MUST provide a global keyboard shortcut to navigate back in the page history.

#### Scenario: Navigate back via Alt+Left
- **WHEN** the user presses `Alt+Left` (Linux/Windows) or `Option+Left` (Mac)
- **THEN** the system SHALL navigate to the previously viewed page
- **AND** the forward history SHALL be preserved for forward navigation

### Requirement: Go Forward Shortcut
The system MUST provide a global keyboard shortcut to navigate forward in the page history.

#### Scenario: Navigate forward via Alt+Right
- **WHEN** the user presses `Alt+Right` (Linux/Windows) or `Option+Right` (Mac)
- **THEN** the system SHALL navigate to the next page in history (if available)
- **AND** the back history SHALL be preserved for back navigation
