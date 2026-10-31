# Spec Delta

## Purpose

Defines the window chrome Cerbo draws itself instead of the operating system's frame: the title bar, window dragging, resizing and the in-app close button.

## ADDED Requirements

### Requirement: Window opens without a native frame

The desktop app's main window SHALL open without native window decorations on every desktop platform, so no system title bar or system close button is shown.

#### Scenario: Start the app
- **WHEN** the user starts the desktop app
- **THEN** the window SHALL have no native title bar
- **THEN** the app's own title bar SHALL be visible at the top

### Requirement: App-drawn title bar

The app SHALL draw a title bar across the full width of the window, above the sidebar and the editor. It SHALL show the active vault name and the open page title. Pressing and moving the pointer on its empty area SHALL move the window, and double-clicking that area SHALL toggle between maximized and restored. Controls inside it SHALL keep working normally and SHALL NOT start a drag.

#### Scenario: Move the window
- **WHEN** the user drags the empty area of the title bar
- **THEN** the window SHALL follow the pointer

#### Scenario: Maximize by double-click
- **WHEN** the user double-clicks the empty area of the title bar
- **THEN** the window SHALL maximize, and a second double-click SHALL restore it

#### Scenario: Title follows navigation
- **WHEN** the user opens the page "Index" in the vault "Notes"
- **THEN** the title bar SHALL show "Notes" and "Index"

### Requirement: Close button inside the app

The title bar SHALL contain a close button at its top-right corner. It SHALL be reachable with the keyboard, SHALL have the accessible name "Quit Cerbo", and SHALL start the application exit flow defined by `application-lifecycle`. It SHALL be the only window button: there SHALL be no minimize or maximize buttons.

#### Scenario: Close button visible
- **WHEN** the app window is open, in any theme and at its minimum width
- **THEN** the close button SHALL be visible at the top-right corner

#### Scenario: Close button is the only window control
- **WHEN** the user looks at the title bar
- **THEN** it SHALL contain no minimize or maximize button

### Requirement: Window stays resizable without a frame

The window SHALL be resizable from all four edges and all four corners while it is not maximized, with the matching resize cursor shown on hover. While the window is maximized, the edges SHALL NOT start a resize.

#### Scenario: Resize from the bottom-right corner
- **WHEN** the user drags the bottom-right corner of a restored window
- **THEN** the window SHALL grow or shrink in both directions

#### Scenario: Edges inactive when maximized
- **WHEN** the window is maximized and the user drags an edge
- **THEN** the window size SHALL NOT change

#### Scenario: Window size is still remembered
- **WHEN** the user resizes the window, quits and restarts the app
- **THEN** the window SHALL open with the saved size

### Requirement: Window outline

While the window is neither maximized nor fullscreen, the app SHALL draw a 1px outline in a theme-specific contrast colour around the whole window, with square corners. While the window is maximized or fullscreen, it SHALL NOT draw the outline.

#### Scenario: Restored window stands out
- **WHEN** the window is restored and overlaps other windows
- **THEN** a visible outline SHALL surround it, in both light and dark themes

#### Scenario: Maximized window has no outline
- **WHEN** the user maximizes the window
- **THEN** the outline SHALL disappear
- **THEN** it SHALL return when the window is restored
