# Custom Title Bar

## Purpose

Provide an optional custom window title bar with theme-aware styling and window controls. When enabled, the custom title bar allows the application to control its window appearance including border/bottom bar colors that adapt to the active theme.

## ADDED Requirements

### Requirement: Custom title bar visibility control

The system SHALL provide a user preference to toggle between native window decorations and custom title bar. The preference SHALL be stored in localStorage and persist across sessions.

#### Scenario: User enables custom title bar
- **WHEN** the user toggles the "Custom Title Bar" setting to enabled
- **THEN** the native decorations SHALL be disabled (`decorations: false`)
- **AND** the custom title bar component SHALL be rendered
- **AND** the preference SHALL be saved to localStorage

#### Scenario: User disables custom title bar
- **WHEN** the user toggles the "Custom Title Bar" setting to disabled
- **THEN** the custom title bar SHALL be removed from the DOM
- **AND** native decorations SHALL be restored (`decorations: true`)
- **AND** the preference SHALL be saved to localStorage

#### Scenario: Application starts with custom title bar enabled
- **WHEN** the application starts
- **AND** localStorage contains `cerbo:useCustomTitleBar: true`
- **THEN** the window SHALL start with `decorations: false`
- **AND** the custom title bar SHALL be rendered immediately

### Requirement: Theme-aware styling

The custom title bar SHALL adapt its colors to match the active theme (light or dark).

#### Scenario: Theme is dark
- **WHEN** the active theme is dark
- **THEN** the title bar background SHALL use `--color-surface` (dark)
- **AND** the bottom border SHALL use `--border` variable

#### Scenario: Theme is light
- **WHEN** the active theme is light
- **THEN** the title bar background SHALL use `--color-surface` (light)
- **AND** the bottom border SHALL use `--border` variable

### Requirement: Window controls

The custom title bar SHALL provide functional window controls for close, minimize, and maximize.

#### Scenario: User clicks close button
- **WHEN** the user clicks the close button
- **THEN** the application SHALL call `window.close()`
- **AND** the application window SHALL close

#### Scenario: User clicks minimize button
- **WHEN** the user clicks the minimize button
- **THEN** the application SHALL call `window.minimize()`
- **AND** the window SHALL be minimized to taskbar/dock

#### Scenario: User clicks maximize button
- **WHEN** the user clicks the maximize button
- **THEN** the application SHALL call `window.toggleMaximize()`
- **AND** the window SHALL toggle between maximized and restored state

### Requirement: Draggable title bar

The title bar area SHALL be draggable to allow window movement.

#### Scenario: User drags title bar
- **WHEN** the user clicks and drags on the title bar (excluding window control buttons)
- **THEN** the application SHALL call `window.startDragging()`
- **AND** the window SHALL follow the mouse movement

### Requirement: Title bar content

The custom title bar SHALL display the application title and window controls.

#### Scenario: Title bar renders
- **WHEN** the custom title bar is enabled
- **THEN** the title bar SHALL display "Cerbo" as the window title
- **AND** window control buttons SHALL be aligned to the opposite side (right on Windows/Linux, left on macOS)