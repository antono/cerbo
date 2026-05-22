# Vault Init

## Purpose
Define the `cerbo init` command that initializes a new vault with `.cerbo/` directory and bundled ontologies.

## Requirements

### Requirement: Init Command
The system SHALL provide `cerbo init` command that initializes a new vault in the current directory. The command SHALL create `.cerbo/` directory with `objects/`, `ontology-map.json`, and bundled ontology objects (Schema.org, FOAF).

#### Scenario: Initialize new vault
- **WHEN** user runs `cerbo init` in an empty directory
- **THEN** `.cerbo/` directory SHALL be created
- **THEN** `.cerbo/objects/` directory SHALL be created
- **THEN** `.cerbo/ontology-map.json` SHALL be created with empty prefixes

#### Scenario: Init creates bundled ontologies
- **WHEN** `cerbo init` completes
- **THEN** a Schema.org ontology object SHALL exist in `.cerbo/objects/<uuid>/`
- **THEN** a FOAF ontology object SHALL exist in `.cerbo/objects/<uuid>/`
- **THEN** `ontology-map.json` SHALL map "schema" to Schema.org UUID
- **THEN** `ontology-map.json` SHALL map "foaf" to FOAF UUID

### Requirement: Index JSON
The system SHALL create `.cerbo/index.json` on `cerbo init` and update it when objects are created/deleted. The file SHALL contain `title_to_uuid` (title→UUID map) and `uuid_to_path` (UUID→relative path map).

#### Scenario: Initial index.json
- **WHEN** `cerbo init` completes
- **THEN** `.cerbo/index.json` SHALL exist
- **THEN** it SHALL contain empty `title_to_uuid` object
- **THEN** it SHALL contain empty `uuid_to_path` object

#### Scenario: Update index on object creation
- **WHEN** a new page object is created with title "My Page" and UUID `<uuid-1>`
- **THEN** `index.json` `title_to_uuid` SHALL map "My Page" to `<uuid-1>`
- **THEN** `index.json` `uuid_to_path` SHALL map `<uuid-1>` to `objects/<uuid-1>/`

### Requirement: Idempotent Init
The `cerbo init` command SHALL be idempotent. Running it on an already-initialized vault SHALL NOT overwrite existing data.

#### Scenario: Re-run init on existing vault
- **WHEN** user runs `cerbo init` on a directory with existing `.cerbo/`
- **THEN** the command SHALL succeed without error
- **THEN** existing objects SHALL NOT be deleted
- **THEN** existing `ontology-map.json` SHALL NOT be overwritten (preserve custom prefixes)
