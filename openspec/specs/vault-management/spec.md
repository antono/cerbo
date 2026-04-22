# Vault Management

## Purpose
Manage vault registration, storage, and retrieval.

## Requirements

### Requirement: Add vault
The system SHALL allow the user to register any existing folder on disk as a vault by providing its filesystem path, either manually or via a native folder selection dialog. A stable UUID SHALL be assigned to the vault at registration time. The vault registry SHALL be stored at `$XDG_CONFIG_HOME/cerbo/vaults.json` and SHALL contain only vault name, path, and ID — no vault-specific data.

#### Scenario: Add a valid folder as vault
- **WHEN** the user provides a path to an existing readable folder (manually or via picker)
- **THEN** the system registers it as a vault with a generated UUID, a user-provided name, and the given path
- **THEN** the vault appears in the vault list

#### Scenario: Add a non-existent folder
- **WHEN** the user provides a path that does not exist on disk
- **THEN** the system SHALL reject the operation with a descriptive error
- **THEN** no vault is added to the registry

#### Scenario: Add a duplicate vault path
- **WHEN** the user provides a path already registered as a vault
- **THEN** the system SHALL reject the operation with a descriptive error

### Requirement: Relocate vault
The system SHALL allow the user to update the path of a registered vault without losing its ID or cache association.

#### Scenario: Update vault path after moving folder
- **WHEN** the user updates the path of an existing vault to a new valid location
- **THEN** the vault ID remains unchanged
- **THEN** the existing cache at `$XDG_CACHE_HOME/cerbo/<vault-id>/` remains valid
- **THEN** the new path is persisted in `vaults.json`

