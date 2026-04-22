## ADDED Requirements

### Requirement: Theme toggle action
The system SHALL provide a mechanism to quickly toggle between light and dark themes. This toggle MUST be accessible via both the UI and a global keyboard shortcut.

#### Scenario: Toggle theme state
- **WHEN** the theme toggle action is triggered
- **AND** the current theme is 'light'
- **THEN** the theme SHALL switch to 'dark'
- **WHEN** the theme toggle action is triggered
- **AND** the current theme is 'dark'
- **THEN** the theme SHALL switch to 'light'
