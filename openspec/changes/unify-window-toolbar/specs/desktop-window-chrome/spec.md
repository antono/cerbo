## MODIFIED Requirements

### Requirement: Window opens without a native frame

The desktop app's main window SHALL open without native window decorations on every desktop platform, so no system title bar or system close button is shown.

#### Scenario: Start the app
- **WHEN** the user starts the desktop app
- **THEN** the window SHALL have no native title bar
- **THEN** the app's own window toolbar, defined by `desktop-window-toolbar`, SHALL be visible at the top

### Requirement: App-drawn title bar

The app's title bar SHALL be the window toolbar defined by `desktop-window-toolbar`, drawn across the full width of the window above the sidebar and the editor. It SHALL show the active vault name and the open page's path. Pressing and moving the pointer on its empty area SHALL move the window, and double-clicking that area SHALL toggle between maximized and restored. Controls inside it SHALL keep working normally and SHALL NOT start a drag.

#### Scenario: Move the window
- **WHEN** the user drags the empty area of the toolbar
- **THEN** the window SHALL follow the pointer

#### Scenario: Maximize by double-click
- **WHEN** the user double-clicks the empty area of the toolbar
- **THEN** the window SHALL maximize, and a second double-click SHALL restore it

#### Scenario: Title follows navigation
- **WHEN** the user opens the page "Index" in the folder "Projects" of the vault "Notes"
- **THEN** the toolbar SHALL show "Notes" in the vault menu and "Projects / Index" as the path
- **THEN** neither name SHALL appear a second time in the toolbar

### Requirement: Close button inside the app

The toolbar SHALL end with a close button at its top-right corner, set apart from the other controls by a separator. It SHALL be reachable with the keyboard, SHALL have the accessible name "Quit Cerbo", and SHALL start the application exit flow defined by `application-lifecycle`. It SHALL be the only window button: there SHALL be no minimize or maximize buttons. No other control in the window SHALL use the same ✕ glyph.

#### Scenario: Close button visible
- **WHEN** the app window is open, in any theme and at its minimum width
- **THEN** the close button SHALL be visible at the top-right corner

#### Scenario: Close button is the only window control
- **WHEN** the user looks at the toolbar
- **THEN** it SHALL contain no minimize or maximize button

#### Scenario: Hiding a panel is not a close button
- **WHEN** the right panel is shown
- **THEN** it SHALL have no ✕ button, and it SHALL be hidden with the toolbar's right panel toggle
