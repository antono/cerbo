## ADDED Requirements

### Requirement: Native folder picker
The system SHALL provide a native OS-level folder selection dialog when the user is adding or relocating a vault. This dialog SHALL allow the user to browse their local filesystem and select an existing directory or create a new directory to serve as a vault root.

#### Scenario: Open native picker
- **WHEN** the user clicks "Select Folder" in the Add Vault UI
- **THEN** a native OS-level folder selection dialog is displayed

#### Scenario: Select folder from native picker
- **WHEN** the user selects a folder in the native dialog and confirms
- **THEN** the native dialog closes
- **THEN** the selected path is automatically filled into the vault path field in the UI

#### Scenario: Create new folder in native picker
- **WHEN** the user creates a new folder within the native dialog, selects it, and confirms
- **THEN** the native dialog closes
- **THEN** the new folder's path is automatically filled into the vault path field in the UI

#### Scenario: Cancel native picker
- **WHEN** the user cancels the native dialog
- **THEN** the native dialog closes
- **THEN** no changes are made to the vault path field in the UI
