# cli-json-output Specification

## Purpose
TBD - created by archiving change cli-json-flag. Update Purpose after archive.
## Requirements
### Requirement: CLI SHALL support --json flag on all commands as per-component option
The CLI SHALL accept a `--json` flag on each command and subcommand individually (not as a global flag). When present, the output SHALL be valid JSON format only, with no additional text, decorations, or log messages. The `--json` flag SHALL appear in the help text for each command that supports it.

#### Scenario: Add --json flag to vault list command
- **WHEN** user runs `cerbo vault list --json`
- **THEN** the output SHALL be a valid JSON array of vault objects with fields: id, name, path
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo vault list --help` SHALL show `--json` option

#### Scenario: Add --json flag to vault add command
- **WHEN** user runs `cerbo vault add "Test" /path/to/vault --json`
- **THEN** the output SHALL be a valid JSON object of the added vault with fields: id, name, path
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo vault add --help` SHALL show `--json` option

#### Scenario: Add --json flag to vault remove command
- **WHEN** user runs `cerbo vault remove <id> --json`
- **THEN** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo vault remove --help` SHALL show `--json` option

#### Scenario: Add --json flag to vault active command
- **WHEN** user runs `cerbo vault active <id> --json`
- **THEN** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo vault active --help` SHALL show `--json` option

#### Scenario: Add --json flag to page list command with optional vault_id
- **WHEN** user runs `cerbo page list --json` (no vault_id)
- **THEN** the system SHALL use the active vault (from state)
- **AND** the output SHALL be a valid JSON array of page objects with fields: slug, title, path
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo page list --help` SHALL show `--json` option and `[VAULT_ID]` as optional

#### Scenario: Add --json flag to page list command with explicit vault_id
- **WHEN** user runs `cerbo page list <vault_id> --json`
- **THEN** the output SHALL be a valid JSON array of page objects with fields: slug, title, path
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to page create command with optional vault_id
- **WHEN** user runs `cerbo page create --json "Page Title"` (no vault_id)
- **THEN** the system SHALL use the active vault
- **AND** the output SHALL be a valid JSON object with field: slug (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo page create --help` SHALL show `--json` option and `vault_id` as optional

#### Scenario: Add --json flag to page read command with optional vault_id
- **WHEN** user runs `cerbo page read --json <slug>` (no vault_id)
- **THEN** the system SHALL use the active vault
- **AND** the output SHALL be a valid JSON object with field: content (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo page read --help` SHALL show `--json` option and `vault_id` as optional

#### Scenario: Add --json flag to page write command with optional vault_id
- **WHEN** user runs `cerbo page write --json <slug> "content"` (no vault_id)
- **THEN** the system SHALL use the active vault
- **AND** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo page write --help` SHALL show `--json` option and `vault_id` as optional

#### Scenario: Add --json flag to page delete command with optional vault_id
- **WHEN** user runs `cerbo page delete --json <slug>` (no vault_id)
- **THEN** the system SHALL use the active vault
- **AND** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo page delete --help` SHALL show `--json` option and `vault_id` as optional

#### Scenario: Add --json flag to page rename command with optional vault_id
- **WHEN** user runs `cerbo page rename --json <slug> "New Title"` (no vault_id)
- **THEN** the system SHALL use the active vault
- **AND** the output SHALL be a valid JSON object with field: new_slug (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo page rename --help` SHALL show `--json` option and `vault_id` as optional

#### Scenario: Add --json flag to attachment list command
- **WHEN** user runs `cerbo page attachment list <vault_id> <slug> --json`
- **THEN** the output SHALL be a valid JSON array of attachment objects with fields: filename, path
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo page attachment list --help` SHALL show `--json` option

#### Scenario: Add --json flag to attachment add command
- **WHEN** user runs `cerbo page attachment add <vault_id> <slug> /path/to/file --json`
- **THEN** the output SHALL be a valid JSON object with field: filename (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo page attachment add --help` SHALL show `--json` option

#### Scenario: Add --json flag to attachment delete command
- **WHEN** user runs `cerbo page attachment delete <vault_id> <slug> <filename> --json`
- **THEN** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo page attachment delete --help` SHALL show `--json` option

#### Scenario: Add --json flag to index build command
- **WHEN** user runs `cerbo index build <vault_id> --json`
- **THEN** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo index build --help` SHALL show `--json` option

#### Scenario: Add --json flag to index backlinks command
- **WHEN** user runs `cerbo index backlinks <vault_id> <slug> --json`
- **THEN** the output SHALL be a valid JSON array of backlink objects with fields: slug, title
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo index backlinks --help` SHALL show `--json` option

#### Scenario: Add --json flag to info command
- **WHEN** user runs `cerbo info --json`
- **THEN** the output SHALL be a valid JSON object with fields: config_dir (string), cache_dir (string), vaults (array of vault summary objects)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo info --help` SHALL show `--json` option

#### Scenario: Watch command with --json flag
- **WHEN** user runs `cerbo watch --json`
- **THEN** each update event SHALL output a valid JSON object with fields: event (string), vault_id (string), message (string)
- **AND** no other text SHALL be printed to stdout
- **AND** `cerbo watch --help` SHALL show `--json` option

#### Scenario: Commands without --json flag remain unchanged
- **WHEN** user runs any command without `--json` flag
- **THEN** the output SHALL remain in the current human-readable format
- **AND** the behavior SHALL NOT change from current implementation

#### Scenario: Invalid JSON output SHALL NOT occur
- **WHEN** a command with `--json` flag encounters an error
- **THEN** the output SHALL be a valid JSON object with fields: error (boolean), message (string), code (optional integer)
- **AND** the exit code SHALL be non-zero

