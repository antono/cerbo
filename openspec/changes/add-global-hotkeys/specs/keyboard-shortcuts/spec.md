## ADDED Requirements

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
