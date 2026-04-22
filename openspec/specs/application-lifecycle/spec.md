# Application Lifecycle

## Purpose
Manage high-level application events such as startup, shutdown, and critical state transitions.

## Requirements

### Requirement: Secure Application Exit
The system MUST provide a secure way to quit the application via a keyboard shortcut that includes a confirmation step to prevent accidental data loss or closure.

#### Scenario: Quit with confirmation
- **WHEN** the user presses `Ctrl+Q` (Linux/Windows) or `Cmd+Q` (Mac)
- **THEN** a confirmation dialog SHALL appear asking "Are you sure you want to quit?"

### Requirement: Mutual Exclusivity of Actions
To reduce cognitive load and prevent UI interference, the system MUST ensure that only one transient UI element (modal, form, or switcher) is active at any given time.

#### Scenario: Triggering a new action while another is active
- **WHEN** a transient UI element is already active (e.g., Vault Switcher is open)
- **AND** the user triggers a different action (e.g., pressing `Ctrl+P` for Search)
- **THEN** the system SHALL automatically close the currently active element
- **AND** the system SHALL then display the newly requested element

### Requirement: Keyboard Navigable Dialogs
All application-level confirmation dialogs MUST be fully navigable using the keyboard.

#### Scenario: Navigating quit dialog
- **WHEN** the quit confirmation dialog is open
- **THEN** the user SHALL be able to switch between "Cancel" and "Quit" buttons using `Arrow` keys or `Tab`
- **THEN** pressing `Enter` SHALL trigger the currently selected action
- **THEN** pressing `Esc` SHALL dismiss the dialog without quitting
