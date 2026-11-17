# Spec Delta

## Purpose

Defines the single toolbar at the top of the desktop window: the one place where every window, vault, page and panel action is reachable with one click.

## ADDED Requirements

### Requirement: One toolbar at the top of the window

The window SHALL have exactly one horizontal bar above its content, spanning the full width. The sidebar, the editor and the right panel SHALL have no header bars of their own. The vault name SHALL appear once in the window, in the toolbar.

#### Scenario: No stacked bars
- **WHEN** the user opens the page "Index" in the vault "Notes" in a wide window
- **THEN** the page tree SHALL start directly below the toolbar
- **THEN** the editor content SHALL start directly below the toolbar
- **THEN** "Notes" SHALL be shown once, in the toolbar

### Requirement: Toolbar control order

In a wide window the toolbar SHALL show, from left to right: the vault menu, the page tree toggle, back, forward, the open page's path with its page menu, the search field, new page, the edit/preview toggle, the right panel toggle, the ⋯ menu, and the close button. A separator SHALL set the close button apart from the other controls.

#### Scenario: Every control present
- **WHEN** a page is open in a window at least 1000px wide
- **THEN** all controls listed above SHALL be visible in that order

#### Scenario: Controls name their hotkey
- **WHEN** the user hovers a toolbar control that has a hotkey
- **THEN** its tooltip SHALL show the action name and the hotkey

### Requirement: Toolbar drags the window

Pressing and moving the pointer on any toolbar area that is not a control SHALL move the window. Double-clicking such an area SHALL toggle maximized and restored. Using a control SHALL NOT start a drag.

#### Scenario: Drag between controls
- **WHEN** the user drags the empty space between the path and the search field
- **THEN** the window SHALL follow the pointer

#### Scenario: Clicking a control does not drag
- **WHEN** the user presses and releases on the new page button
- **THEN** the New Page dialog SHALL open and the window SHALL NOT move

### Requirement: Vault menu

The vault control SHALL show the Cerbo icon and the active vault name. Clicking it SHALL open the vault selector defined by `vault-selector`.

#### Scenario: Switch vault from the toolbar
- **WHEN** the user clicks "Notes ▾" and chooses the vault "Work"
- **THEN** "Work" SHALL become the active vault and the toolbar SHALL show "Work"

#### Scenario: No vault yet
- **WHEN** no vault is registered
- **THEN** the vault control SHALL read "Add vault" and clicking it SHALL start the add-vault flow

### Requirement: Page path and page menu

The toolbar SHALL show the open page's virtual path. Folder segments SHALL be muted, and the page title SHALL open a page menu. The menu SHALL offer, with their hotkeys: Rename (`r`), Move (`m`), Copy link, Reveal in tree, and Delete (`Del`), with Delete last and marked as destructive.

#### Scenario: Rename from the page menu
- **WHEN** the user opens the page menu of "Projects / Index" and picks Rename
- **THEN** the Rename dialog SHALL open for "Index"

#### Scenario: Folder segment reveals the folder
- **WHEN** the user clicks the "Projects" segment
- **THEN** the page tree SHALL show and select the folder "Projects", expanding its parents

#### Scenario: No open page
- **WHEN** no page is open
- **THEN** the path and the page menu SHALL be absent

### Requirement: Save state in the toolbar

While the open page has unsaved changes or is being saved, a small dot SHALL appear after the page title in the toolbar. The floating "Saving…" label over the editor SHALL NOT be shown.

#### Scenario: Typing shows the dot
- **WHEN** the user types in the editor
- **THEN** the dot SHALL appear, and it SHALL disappear once the page is saved

### Requirement: Search field

The toolbar SHALL contain a search field showing the placeholder "Search pages or > command" and the hotkey `Ctrl+P`. Clicking it SHALL open page search as defined by `keyboard-shortcuts` and `command-palette`.

#### Scenario: Open search by click
- **WHEN** the user clicks the search field
- **THEN** page search SHALL open with the cursor in its input

### Requirement: Panel toggles

The toolbar SHALL have one toggle for the page tree and one for the right panel. Each toggle SHALL look pressed while its panel is shown. Panel visibility SHALL be remembered across restarts. The right panel SHALL have no close button of its own.

#### Scenario: Hide and restore the right panel
- **WHEN** the user clicks the right panel toggle twice
- **THEN** the right panel SHALL hide and then show again

#### Scenario: Hidden tree survives restart
- **WHEN** the user hides the page tree, quits and restarts the app
- **THEN** the page tree SHALL still be hidden

### Requirement: Edit and preview toggle

The toolbar SHALL contain one button that switches the open page between edit and preview. It SHALL show an eye in edit mode and a pencil in preview mode. The editor SHALL have no mode button of its own.

#### Scenario: Toggle mode
- **WHEN** a page is open in preview and the user clicks the toggle
- **THEN** the page SHALL switch to edit mode with the caret restored

### Requirement: Overflow menu

The ⋯ menu SHALL contain, each with its hotkey where one exists: the theme switch (labelled with the theme it switches to), SPARQL console, New folder, Keyboard shortcuts, and Quit. No other toolbar control SHALL duplicate these entries.

#### Scenario: Open SPARQL console by mouse
- **WHEN** the user opens ⋯ and picks "SPARQL console"
- **THEN** the SPARQL console SHALL open

#### Scenario: Theme entry names the target theme
- **WHEN** the dark theme is active and the user opens ⋯
- **THEN** the theme entry SHALL read "Light theme" and show `Ctrl+T`

### Requirement: Toolbar menus work from the keyboard

The vault, page and ⋯ menus SHALL support the arrow keys and `j`/`k` to move, `Enter` to pick and `Esc` to close. They SHALL follow the one-transient-element rule of `application-lifecycle`.

#### Scenario: Pick with Enter
- **WHEN** the ⋯ menu is open and the user presses ↓ twice and Enter
- **THEN** the third entry's action SHALL run and the menu SHALL close
