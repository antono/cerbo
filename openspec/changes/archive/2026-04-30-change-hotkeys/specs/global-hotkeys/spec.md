## MODIFIED Requirements

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
