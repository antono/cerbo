## ADDED Requirements

### Requirement: Shortcuts Help Modal
The system MUST provide a modal dialog that lists all available application-level keyboard shortcuts and their descriptions.

#### Scenario: Display shortcuts help
- **WHEN** the Shortcuts Help action is triggered (via `F1`)
- **THEN** a modal dialog SHALL be displayed
- **AND** the dialog SHALL contain a clear list of all registered global shortcuts (e.g., Ctrl+P, Ctrl+N, Ctrl+T, Ctrl+O, F1)
- **AND** each shortcut SHALL have a brief description of its action

### Requirement: Modal Dismissal
The Shortcuts Help modal MUST be dismissible via the standard 'Esc' key or by clicking outside the modal area.

#### Scenario: Close help modal
- **WHEN** the Shortcuts Help modal is active
- **AND** the user presses `Esc`
- **THEN** the modal SHALL close
