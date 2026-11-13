# Spec Delta

## MODIFIED Requirements

### Requirement: Secure Application Exit
The system MUST provide a secure way to quit the application via a keyboard shortcut, and via the close button in the app's title bar, both of which include the same confirmation step to prevent accidental data loss or closure. Both SHALL use the same exit path, which saves UI settings and window size before the process ends.

#### Scenario: Quit with confirmation
- **WHEN** the user presses `Ctrl+Q` (Linux/Windows) or `Cmd+Q` (Mac)
- **THEN** a confirmation dialog SHALL appear asking "Are you sure you want to quit?"

#### Scenario: Close button asks for confirmation
- **WHEN** the user clicks the close button in the title bar
- **THEN** the same confirmation dialog SHALL appear
- **WHEN** the user confirms
- **THEN** UI settings and the window size SHALL be saved and the app SHALL exit

#### Scenario: Close cancelled
- **WHEN** the user clicks the close button and then chooses "Cancel"
- **THEN** the app SHALL keep running with nothing changed
