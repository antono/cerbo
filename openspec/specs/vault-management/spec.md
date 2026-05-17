# Vault Management

## Purpose
Manage vault initialization and registration using `cerbo init` command.

## Requirements

### Requirement: Initialize vault
The system SHALL provide `cerbo init` command that initializes a new vault in the current directory. The command SHALL create `.cerbo/` directory with `objects/`, `index.json`, and `ontology-map.json`. The command SHALL create bundled ontology objects (Schema.org, FOAF).

#### Scenario: Initialize new vault
- **WHEN** user runs `cerbo init` in an empty directory
- **THEN** `.cerbo/` directory is created
- **THEN** `.cerbo/objects/` directory is created
- **THEN** `.cerbo/index.json` is created with empty title_to_uuid and uuid_to_path
- **THEN** `.cerbo/ontology-map.json` is created
- **THEN** Schema.org ontology object is created in `.cerbo/objects/<uuid>/`
- **THEN** FOAF ontology object is created in `.cerbo/objects/<uuid>/`

#### Scenario: Re-run init on existing vault
- **WHEN** user runs `cerbo init` on a directory with existing `.cerbo/`
- **THEN** the command SHALL succeed without error
- **THEN** existing objects SHALL NOT be deleted
- **THEN** existing `ontology-map.json` SHALL NOT be overwritten

### Requirement: Vault is directory with .cerbo/
The system SHALL treat any directory containing `.cerbo/` as a valid vault. The vault root is the parent directory of `.cerbo/`.

#### Scenario: Detect vault
- **WHEN** the system scans for vaults
- **THEN** it looks for directories containing `.cerbo/` subdirectory
- **THEN** the vault name is derived from the directory name (or from `meta.ttl` if available)
