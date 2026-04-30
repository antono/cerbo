# Global Hotkeys

## Purpose
Define how application-level hotkeys behave when transient UI is active.
## Requirements
### Requirement: Add Vault Shortcut Registration
The application MUST register `Ctrl+Shift+O` (or `Cmd+Shift+O` on Mac) as the global hotkey for adding a new vault.

#### Scenario: Register Add Vault hotkey
- **WHEN** the application initializes global hotkeys
- **THEN** it SHALL register `Ctrl+Shift+O` (Linux/Windows) or `Cmd+Shift+O` (Mac) to trigger the Add Vault workflow

### Requirement: Vault Selector Shortcut Registration
The application MUST register `Ctrl+O` (or `Cmd+O` on Mac) as the global hotkey for opening the vault selector.

#### Scenario: Register Vault Selector hotkey
- **WHEN** the application initializes global hotkeys
- **THEN** it SHALL register `Ctrl+O` (Linux/Windows) or `Cmd+O` (Mac) to open the vault selector

### Requirement: Confirmation modal hotkey suppression
When a modal confirmation dialog is open, the application MUST suppress all global hotkeys except the confirmation dialog's own navigation and dismissal keys.

#### Scenario: Suppress hotkeys during confirmation
- **WHEN** a confirmation modal is open
- **THEN** all global hotkeys except navigation/dismissal keys SHALL be suppressed

### Requirement: Confirmation modal active-state gating
The application MUST treat confirmation modal open state as a blocking condition for layout-level shortcut handling.

#### Scenario: Block layout shortcuts during confirmation
- **WHEN** a confirmation modal is open
- **THEN** layout-level shortcut handling SHALL be blocked

### Requirement: Non-confirmation shortcuts preserved
Global hotkeys outside modal confirmation state MUST continue to work unchanged.

#### Scenario: Hotkeys work when no confirmation active
- **WHEN** no confirmation modal is open
- **THEN** all global hotkeys SHALL continue to work normally

