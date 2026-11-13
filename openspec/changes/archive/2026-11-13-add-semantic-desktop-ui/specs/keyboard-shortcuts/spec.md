# Spec Delta

## ADDED Requirements

### Requirement: SPARQL Console Shortcut

The system SHALL open the SPARQL console when the user presses `Ctrl+Shift+E` (Linux/Windows) or `Cmd+Shift+E` (Mac), including while the editor has focus, unless a confirmation modal is open. The shortcuts help SHALL list it.

#### Scenario: Open console via Ctrl+Shift+E
- **WHEN** the user presses `Ctrl+Shift+E` (Linux/Windows) or `Cmd+Shift+E` (Mac)
- **THEN** the SPARQL console SHALL open

#### Scenario: Listed in shortcuts help
- **WHEN** the user opens the shortcuts help
- **THEN** it SHALL list `Ctrl+Shift+E` as "SPARQL console"
