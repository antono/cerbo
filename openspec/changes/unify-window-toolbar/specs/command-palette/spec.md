# Spec Delta

## Purpose

Lets the user find and run any app action by name from the page search window, so every feature is one hotkey away even when it has no button of its own.

## ADDED Requirements

### Requirement: Command mode in page search

When the page search input starts with `>`, the search window SHALL list app actions instead of pages. Deleting the `>` SHALL return to page results.

#### Scenario: Switch to commands
- **WHEN** page search is open and the user types `>`
- **THEN** the list SHALL show app actions instead of pages

#### Scenario: Back to pages
- **WHEN** the input is `>the` and the user deletes the `>`
- **THEN** the list SHALL show pages matching "the"

### Requirement: Open straight in command mode

`Ctrl+Shift+P` (`Cmd+Shift+P` on Mac) SHALL open page search with `>` already typed. Pressing it while search is open SHALL close it.

#### Scenario: Open commands by hotkey
- **WHEN** the user presses `Ctrl+Shift+P`
- **THEN** search SHALL open with `>` in the input and the action list shown

### Requirement: Every action is listed

The action list SHALL include every action offered in the toolbar and its menus, plus "Go back", "Go forward", "Add vault", "Next page" and "Previous page". Each entry SHALL show its name, its icon and its hotkey if it has one. Actions that do not apply right now SHALL be left out.

#### Scenario: Hotkey shown
- **WHEN** the user types `>sparql`
- **THEN** "SPARQL console" SHALL be listed with `Ctrl+Shift+E`

#### Scenario: Page actions need a page
- **WHEN** no page is open and the user types `>`
- **THEN** "Rename page", "Move page" and "Delete page" SHALL NOT be listed

### Requirement: Action matching

Actions SHALL be matched when the query, ignoring case, is a substring of their name or of one of their listed synonyms. "theme", "dark" and "light" SHALL all find the theme switch. With an empty query after `>`, the most recently run actions SHALL come first.

#### Scenario: Synonym match
- **WHEN** the user types `>dark` while the light theme is active
- **THEN** "Dark theme" SHALL be the first entry

### Requirement: Running an action

`Enter` or a click SHALL close the search window and then run the chosen action, exactly as its button or hotkey would. Actions that open a dialog SHALL open it with focus in it.

#### Scenario: Run new folder
- **WHEN** the user types `>folder` and presses Enter
- **THEN** the search window SHALL close and the New Folder dialog SHALL open with focus in its path field
