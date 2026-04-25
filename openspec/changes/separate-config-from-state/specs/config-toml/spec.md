# Config TOML

## ADDED Requirements

### Requirement: Config stored in TOML format
The system SHALL store persistent configuration in TOML format at `$XDG_CONFIG_DIR/cerbo/vaults.toml`.

#### Scenario: Config file exists
- **WHEN** vaults.toml exists at `$XDG_CONFIG_DIR/cerbo/vaults.toml`
- **THEN** system SHALL load configuration from TOML file

#### Scenario: Config file missing
- **WHEN** vaults.toml does not exist
- **THEN** system SHALL initialize with empty config (valid empty state)

#### Scenario: Config file corrupted
- **WHEN** vaults.toml contains invalid TOML
- **THEN** system SHALL log parse error and initialize empty config (not crash)

### Requirement: Vaults stored in config.toml
The system SHALL store the list of registered vaults in the `vaults` array of vaults.toml.

#### Scenario: Add vault
- **WHEN** user adds a vault with path `/data/notes`
- **THEN** vault appears in vaults.toml vaults array with unique id, name, and path

#### Scenario: Remove vault
- **WHEN** user removes a vault
- **THEN** vault is removed from vaults.toml vaults array

#### Scenario: Set active vault
- **WHEN** user selects a vault as active
- **THEN** vault id is stored in vaults.toml

### Requirement: Config persisted atomically
The system SHALL write vaults.toml atomically using rename(2) to avoid corruption.

#### Scenario: Write config
- **WHEN** config changes need saving
- **THEN** system SHALL write to temp file first, then rename to vaults.toml

## MODIFIED Requirements

### Requirement: Vault discovery
**Original**: Vaults persisted in JSON format.

**Updated**: Vaults persisted in TOML format with same data model.

#### Scenario: Load vaults from TOML
- **WHEN** system loads vaults from vaults.toml
- **THEN** returns vault list equivalent to previous JSON format

#### Scenario: Per-vault last_open_page stored in state
- **WHEN** user opens a page in vault
- **THEN** last_open_page persisted to state.toml (NOT config.toml)
