## ADDED Requirements

### Requirement: Theme toggle visual feedback
The theme toggle component SHALL provide visual feedback reflecting the current active theme.

#### Scenario: Light mode active
- **WHEN** the light theme is active
- **THEN** the toggle button SHALL display a Sun icon

#### Scenario: Dark mode active
- **WHEN** the dark theme is active
- **THEN** the toggle button SHALL display a Moon icon

### Requirement: Modal-like button behavior
The theme toggle SHALL be a single button that cycles through themes or opens a selection menu (per user request: "single modal button").

#### Scenario: Toggling theme
- **WHEN** the user clicks the theme toggle button
- **THEN** the system SHALL switch to the next theme in the sequence (Light -> Dark -> Light)
