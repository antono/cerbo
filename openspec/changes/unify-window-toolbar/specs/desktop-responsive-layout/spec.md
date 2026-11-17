# Spec Delta

## Purpose

Defines how the page tree, the editor and the right panel share the window at different widths, so the editor stays usable from a full-screen window down to a phone-width one.

## ADDED Requirements

### Requirement: Width classes

The layout SHALL follow the current window width: wide at 1000px and above, medium from 700px to 999px, and narrow below 700px. The layout SHALL change as soon as the window is resized across a boundary, without a reload.

#### Scenario: Resize across a boundary
- **WHEN** the user shrinks the window from 1200px to 800px
- **THEN** the layout SHALL switch from wide to medium while the user drags

### Requirement: Minimum window size

The window SHALL NOT be resizable below 360 by 480 pixels.

#### Scenario: Shrink past the minimum
- **WHEN** the user drags the window corner toward 200 by 200 pixels
- **THEN** the window SHALL stop at 360 by 480

### Requirement: Editor keeps its width

The editor SHALL never be narrower than 360px, or than the whole window when the window is narrower than that. Side panels SHALL give way to the editor, not the other way round.

#### Scenario: Phone-width window shows the page
- **WHEN** a page is open and the window is 390px wide
- **THEN** the page content SHALL fill the width under the toolbar
- **THEN** no part of the tree or the right panel SHALL cover or squeeze it until the user opens one

### Requirement: Wide layout

In the wide layout, the shown panels SHALL sit side by side with the editor and keep their resizable widths.

#### Scenario: All three columns
- **WHEN** both panels are shown in a 1200px window
- **THEN** the tree, the editor and the right panel SHALL be side by side

### Requirement: Medium layout

In the medium layout, the page tree SHALL stay docked when shown, and the right panel SHALL open over the editor from the right edge instead of taking space from it. The toolbar search field SHALL shrink to an icon, and folder segments of the path SHALL collapse to "…".

#### Scenario: Right panel floats
- **WHEN** the window is 800px wide and the user turns the right panel on
- **THEN** the panel SHALL cover the right part of the editor without changing the editor's width

#### Scenario: Collapsed path
- **WHEN** the window is 800px wide and "Projects / Index" is open
- **THEN** the toolbar SHALL show "… / Index", and clicking "…" SHALL list the folder segments

### Requirement: Narrow layout

In the narrow layout, only the editor SHALL be docked. The page tree SHALL open as a drawer from the left, over the editor, with the vault menu and new page at its top. The right panel SHALL open as a sheet from the bottom, with Backlinks, Similar and Files as tabs. Toolbar and menu targets SHALL be at least 32px. The edit/preview toggle SHALL float at the bottom-right of the editor.

#### Scenario: Open a page from the drawer
- **WHEN** the window is 390px wide and the user taps ☰ and then "Inbox"
- **THEN** "Inbox" SHALL open and the drawer SHALL close

#### Scenario: Right panel as a sheet
- **WHEN** the window is 390px wide and the user opens the right panel from ⋯
- **THEN** a sheet with the Backlinks, Similar and Files tabs SHALL slide up from the bottom

### Requirement: Narrow toolbar

In the narrow layout the toolbar SHALL show only: the tree toggle (☰), back, the page title with its page menu, search, ⋯ and the close button. Forward, new page, the right panel toggle and the mode toggle SHALL move into ⋯ or the drawer.

#### Scenario: Narrow toolbar fits
- **WHEN** a page with a 60-character title is open in a 360px window
- **THEN** every narrow toolbar control SHALL be visible and the title SHALL be truncated with an ellipsis

### Requirement: Overlays close predictably

A drawer, a floating panel or a sheet SHALL close on `Esc`, on a click outside it, and when its own toggle is used again. Opening one SHALL close any other.

#### Scenario: Close the drawer by clicking outside
- **WHEN** the tree drawer is open and the user clicks the editor
- **THEN** the drawer SHALL close and the click SHALL NOT edit the page

### Requirement: Docked panel state is kept per width class

Showing or hiding a panel in the wide layout SHALL be remembered across restarts. An overlay opened in the medium or narrow layout SHALL NOT change that remembered state.

#### Scenario: Back to wide
- **WHEN** the right panel is shown in the wide layout, the user narrows the window, then widens it again
- **THEN** the right panel SHALL be docked and shown again
