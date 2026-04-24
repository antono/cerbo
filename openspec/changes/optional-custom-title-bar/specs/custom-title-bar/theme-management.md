# Theme Management

## Purpose
Manage application themes (light/dark), persistence, and system synchronization.

## Requirements

### Requirement: Theme state persistence
The system SHALL persist the user's theme preference (light or dark) across application restarts.

#### Scenario: User selects dark theme
- **WHEN** the user selects the dark theme
- **THEN** the dark theme is applied and saved to persistent storage

#### Scenario: Application loads with saved theme
- **WHEN** the application starts
- **THEN** it SHALL load and apply the previously saved theme preference

### Requirement: System theme synchronization
The system SHALL support an option to synchronize with the operating system's theme preference.

#### Scenario: Sync with system theme
- **WHEN** the user chooses the system theme option
- **THEN** the application SHALL update its theme whenever the system theme changes

### Requirement: Editor theme consistency
The markdown editor component SHALL dynamically adapt its styling—including background colors, text colors, icons, and syntax highlighting—to match the active application theme (light or dark).

#### Scenario: Editor in dark mode
- **WHEN** the application is in dark mode
- **THEN** the editor SHALL use a dark background with light, high-contrast text
- **AND** all editor icons SHALL remain clearly visible and distinguishable

### Requirement: Theme toggle action
The system SHALL provide a mechanism to quickly toggle between light and dark themes. This toggle MUST be accessible via both the UI and a global keyboard shortcut.

#### Scenario: Toggle theme state
- **WHEN** the theme toggle action is triggered
- **AND** the current theme is 'light'
- **THEN** the theme SHALL switch to 'dark'
- **WHEN** the theme toggle action is triggered
- **AND** the current theme is 'dark'
- **THEN** the theme SHALL switch to 'light'

### Requirement: Custom title bar theme synchronization MODIFIED

The system SHALL synchronize theme changes with the custom title bar when enabled. When custom title bar is enabled in settings, changing themes SHALL also update the title bar styling.

#### Scenario: Theme change with custom title bar enabled
- **WHEN** the theme toggles
- **AND** custom title bar is enabled (localStorage `cerbo:useCustomTitleBar: true`)
- **THEN** the custom title bar SHALL update to reflect the new theme colors

#### Scenario: Theme change with custom title bar disabled
- **WHEN** the theme toggles
- **AND** custom title bar is disabled (localStorage `cerbo:useCustomTitleBar: false` or missing)
- **THEN** the title bar styling SHALL NOT change (relies on native decorations)