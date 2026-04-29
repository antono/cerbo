## MODIFIED Requirements

### Requirement: Shortcuts Help Modal
The system MUST provide a modal dialog that lists all available application-level keyboard shortcuts and their descriptions.

#### Scenario: Display shortcuts help on narrow screens
- **WHEN** the Shortcuts Help action is triggered (via `F1`)
- **THEN** a modal dialog SHALL be displayed
- **AND** the dialog SHALL contain a clear list of all registered global shortcuts and their descriptions in a single-column layout

#### Scenario: Display shortcuts help on wide screens
- **WHEN** the Shortcuts Help action is triggered (via `F1`)
- **AND** the viewport is wide enough for the responsive breakpoint
- **THEN** the modal dialog SHALL display in a wider shell
- **AND** the shortcuts list SHALL display in two columns
- **AND** related shortcuts SHALL be grouped by meaning
- **AND** each shortcut SHALL retain its description

### Requirement: Modal Dismissal
The Shortcuts Help modal MUST be dismissible via the standard 'Esc' key or by clicking outside the modal area.

#### Scenario: Close help modal
- **WHEN** the Shortcuts Help modal is active
- **AND** the user presses `Esc`
- **THEN** the modal SHALL close

#### Scenario: Close help modal by backdrop click
- **WHEN** the Shortcuts Help modal is active
- **AND** the user clicks outside the modal content
- **THEN** the modal SHALL close
