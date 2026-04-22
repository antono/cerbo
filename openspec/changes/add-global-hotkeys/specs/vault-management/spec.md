## MODIFIED Requirements

### Requirement: Add vault
The system SHALL provide a simplified workflow for registering a new vault. Triggering the "Add Vault" action (via UI button or `Ctrl+O` / `Cmd+O` hotkey) SHALL immediately trigger a native system directory picker. Upon selection, the system SHALL automatically register the folder as a vault, deriving the initial vault name from the folder's name. A stable UUID SHALL be assigned to the vault at registration time.

#### Scenario: Add a vault via native picker
- **WHEN** the user triggers the "Add Vault" action
- **THEN** the native system directory picker is displayed
- **WHEN** the user selects a valid folder
- **THEN** the system registers it as a vault with a generated UUID
- **AND** the system SHALL use the folder's name as the vault name by default
- **AND** the vault appears in the vault list

#### Scenario: Add a duplicate vault path
- **WHEN** the user provides a path already registered as a vault
- **THEN** the system SHALL reject the operation with a descriptive error
