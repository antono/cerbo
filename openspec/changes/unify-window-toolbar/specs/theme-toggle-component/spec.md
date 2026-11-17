## MODIFIED Requirements

### Requirement: Theme toggle visual feedback
The theme switch SHALL be an entry in the window toolbar's ⋯ menu. It SHALL be labelled with the theme it switches to and show that theme's icon from the `icon-system`.

#### Scenario: Light mode active
- **WHEN** the light theme is active
- **THEN** the entry SHALL read "Dark theme" and display the `Moon` icon

#### Scenario: Dark mode active
- **WHEN** the dark theme is active
- **THEN** the entry SHALL read "Light theme" and display the `Sun` icon

### Requirement: Modal-like button behavior
The theme switch SHALL be a single menu entry that switches to the other theme and closes the menu.

#### Scenario: Toggling theme
- **WHEN** the user picks the theme entry in the ⋯ menu
- **THEN** the system SHALL switch to the next theme in the sequence (Light -> Dark -> Light)
- **THEN** the menu SHALL close
