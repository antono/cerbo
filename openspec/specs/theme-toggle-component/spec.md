# Theme Toggle Component

## Purpose
Provide a UI component for users to manually switch between themes.

## Requirements

### Requirement: Theme toggle visual feedback
The theme toggle component SHALL provide visual feedback reflecting the current active theme using standardized icon components.

#### Scenario: Light mode active
- **WHEN** the light theme is active
- **THEN** the toggle button SHALL display the `Sun` icon from the `icon-system`

#### Scenario: Dark mode active
- **WHEN** the dark theme is active
- **THEN** the toggle button SHALL display the `Moon` icon from the `icon-system`

### Requirement: Modal-like button behavior
The theme toggle SHALL be a single button that cycles through themes or opens a selection menu (per user request: "single modal button").

#### Scenario: Toggling theme
- **WHEN** the user clicks the theme toggle button
- **THEN** the system SHALL switch to the next theme in the sequence (Light -> Dark -> Light)
