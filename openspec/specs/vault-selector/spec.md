# Vault Selector

## Purpose
Provide a modal vault selector for switching, adding, and managing vault access from the keyboard.

## Requirements

### Requirement: Keyboard-triggered vault selector
The application MUST toggle a vault selector when the user presses `Ctrl+O` (`Cmd+O` on macOS). `Ctrl+Shift+O` (`Cmd+Shift+O`) MUST start adding a vault directly.

#### Scenario: Open the selector
- **WHEN** the user presses `Ctrl+O`
- **THEN** the vault selector SHALL open, and pressing `Ctrl+O` again SHALL close it

#### Scenario: Add a vault from the keyboard
- **WHEN** the user presses `Ctrl+Shift+O`
- **THEN** the add-vault flow SHALL start without opening the selector

### Requirement: Vault selector behavior
The vault selector MUST present available vaults in a modal selector UI consistent with the existing page selector interaction model.

#### Scenario: Keyboard navigation
- **WHEN** the selector is open with three registered vaults
- **THEN** the active vault SHALL be preselected, and the arrow keys SHALL move the selection between the vaults

### Requirement: Vault switching
Selecting a vault in the selector MUST open that vault using the existing vault open flow.

#### Scenario: Switch vault
- **WHEN** the user selects another vault and confirms with `Enter` or a click
- **THEN** the selector SHALL close and the chosen vault SHALL open

### Requirement: Add vault from selector
The vault selector MUST provide a way to add a new vault.

#### Scenario: Add vault entry
- **WHEN** the user chooses "Add vault" in the selector
- **THEN** the selector SHALL close and the add-vault flow SHALL start

### Requirement: Replace old switcher
The old inline vault switcher MUST no longer be shown in the sidebar header.

#### Scenario: Sidebar header
- **WHEN** a vault is open
- **THEN** the sidebar header SHALL show the vault name as a button that opens the selector, and no inline vault list

### Requirement: Vault entries show the Cerbo icon

Each vault entry in the vault selector SHALL show the Cerbo app icon (the brain-on-chip design, the same artwork as the app and window icon), scaled down to the entry's line height, before the vault name. The icon SHALL be decorative: it SHALL NOT change the entry's accessible name. It SHALL stay legible in both light and dark themes.

#### Scenario: Every vault has the icon
- **WHEN** the user opens the vault selector with three registered vaults
- **THEN** each of the three entries SHALL show the Cerbo icon before its name

#### Scenario: Icon matches the app icon
- **WHEN** the app icon artwork changes in `src/favicon.svg`
- **THEN** the vault selector SHALL show the new artwork without a separate copy to update

#### Scenario: Screen readers read only the name
- **WHEN** a screen reader focuses a vault entry named "Notes"
- **THEN** it SHALL announce "Notes" and nothing for the icon

#### Scenario: Dark theme
- **WHEN** the app uses the dark theme
- **THEN** the icon's outline SHALL remain distinguishable from the selector background
