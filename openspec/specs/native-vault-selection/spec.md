# Native Vault Selection

## Purpose
Provide a native OS-level folder selection dialog for vault management.

## Requirements

### Requirement: Native folder picker
The system SHALL provide a native OS-level folder selection dialog that immediately triggers upon clicking "Add Vault". This dialog SHALL allow the user to select an existing directory to serve as a vault root.

#### Scenario: Open native picker
- **WHEN** the user clicks "Add Vault" in the Vault Switcher
- **THEN** a native OS-level folder selection dialog is immediately displayed

#### Scenario: Select folder from native picker
- **WHEN** the user selects a folder in the native dialog and confirms
- **THEN** the native dialog closes
- **THEN** the system registers the selected path as a new vault and automatically derives the vault name from the folder

#### Scenario: Cancel native picker
- **WHEN** the user cancels the native dialog
- **THEN** the native dialog closes
- **THEN** no changes are made to the vault list
