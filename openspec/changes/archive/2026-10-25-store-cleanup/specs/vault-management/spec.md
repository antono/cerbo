# Vault Management

## MODIFIED Requirements

### Requirement: Initialize vault
The system SHALL provide `cerbo init` command that initializes a new vault in the current
directory. The command SHALL create `.cerbo/` directory with `objects/` and
`ontology-map.json`. The command SHALL create bundled ontology objects (Schema.org, FOAF).
The command SHALL ensure that the vault root contains a `.gitignore` file with a line of
exactly `/cerbo/`, so the materialised symlink tree (see the `vault-symlink` capability) is
never committed, and a line of exactly `/.cerbo/trash/`, so deleted objects are not
committed.

#### Scenario: Initialize new vault
- **WHEN** user runs `cerbo init` in an empty directory
- **THEN** `.cerbo/` directory is created
- **THEN** `.cerbo/objects/` directory is created
- **THEN** `.cerbo/ontology-map.json` is created
- **THEN** Schema.org ontology object is created in `.cerbo/objects/<uuid>/`
- **THEN** FOAF ontology object is created in `.cerbo/objects/<uuid>/`
- **THEN** `.gitignore` is created at the vault root containing the line `/cerbo/`
- **THEN** `.gitignore` contains the line `/.cerbo/trash/`
- **THEN** no `.cerbo/index.json` is created

#### Scenario: Re-run init on existing vault
- **WHEN** user runs `cerbo init` on a directory with existing `.cerbo/`
- **THEN** the command SHALL succeed without error
- **THEN** existing objects SHALL NOT be deleted
- **THEN** existing `ontology-map.json` SHALL NOT be overwritten

#### Scenario: Append `/cerbo/` to existing `.gitignore`
- **WHEN** user runs `cerbo init` in a directory where `.gitignore` exists and does NOT already contain a line exactly matching `/cerbo/`
- **THEN** the command appends a section to `.gitignore` containing a one-line comment (e.g. `# Cerbo symlink tree (regenerate with: cerbo symlink)`) followed by the line `/cerbo/`
- **THEN** prior `.gitignore` contents SHALL NOT be modified or reordered

#### Scenario: `.gitignore` already has the entry
- **WHEN** user runs `cerbo init` in a directory where `.gitignore` already contains a line exactly matching `/cerbo/`
- **THEN** `.gitignore` SHALL NOT be modified

#### Scenario: Init on a vault containing a legacy index.json
- **WHEN** user runs `cerbo init` on a vault that already contains `.cerbo/index.json`
- **THEN** the command SHALL succeed
- **THEN** the file SHALL be left untouched and unread
