## MODIFIED Requirements

### Requirement: Hotkeys help entry point
The application MUST expose a "Keyboard shortcuts" entry, with its `F1` hotkey, in the window toolbar's ⋯ menu and in command mode of page search. Picking it SHALL open keyboard shortcuts help.

#### Scenario: Open help from the menu
- **WHEN** the user opens the ⋯ menu and picks "Keyboard shortcuts"
- **THEN** the keyboard shortcuts help SHALL open

### Requirement: Hotkeys help content
The keyboard shortcuts help MUST include the vault selector shortcut, the panel toggles, the edit/preview toggle and the command mode shortcut.

#### Scenario: New shortcuts listed
- **WHEN** the keyboard shortcuts help is open
- **THEN** it SHALL list `Ctrl+O`, `Ctrl+\`, `Ctrl+Shift+\`, `Ctrl+E` and `Ctrl+Shift+P` with their actions
