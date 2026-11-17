## ADDED Requirements

### Requirement: Page tree toggle shortcut
The system MUST provide `Ctrl+\` (`Cmd+\` on Mac) to show or hide the page tree, also while the editor has focus.

#### Scenario: Hide the tree from the editor
- **WHEN** the user is typing in the editor and presses `Ctrl+\`
- **THEN** the page tree SHALL hide and the caret SHALL stay where it was

### Requirement: Right panel toggle shortcut
The system MUST provide `Ctrl+Shift+\` (`Cmd+Shift+\` on Mac) to show or hide the right panel, also while the editor has focus.

#### Scenario: Show the right panel
- **WHEN** the right panel is hidden and the user presses `Ctrl+Shift+\`
- **THEN** the right panel SHALL be shown

### Requirement: Edit and preview shortcut
The system MUST provide `Ctrl+E` (`Cmd+E` on Mac) to switch the open page between edit and preview, in both directions. The existing `i` and `Esc` mode keys SHALL keep working.

#### Scenario: Toggle twice
- **WHEN** a page is open in preview and the user presses `Ctrl+E` twice
- **THEN** the page SHALL be in edit mode after the first press and in preview after the second

### Requirement: Command mode shortcut
The system MUST provide `Ctrl+Shift+P` (`Cmd+Shift+P` on Mac) to open page search in command mode, as defined by `command-palette`.

#### Scenario: Open command mode
- **WHEN** the user presses `Ctrl+Shift+P`
- **THEN** page search SHALL open with `>` in its input
