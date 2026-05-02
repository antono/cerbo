## ADDED Requirements

### Requirement: CLI SHALL support --json flag on all commands
The CLI SHALL accept a `--json` flag on all commands and subcommands. When present, the output SHALL be valid JSON format only, with no additional text, decorations, or log messages.

#### Scenario: Add --json flag to vault list command
- **WHEN** user runs `cerbo vault list --json`
- **THEN** the output SHALL be a valid JSON array of vault objects with fields: id, name, path, active
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to vault add command
- **WHEN** user runs `cerbo vault add "Test" /path/to/vault --json`
- **THEN** the output SHALL be a valid JSON object of the added vault with fields: id, name, path, active
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to vault remove command
- **WHEN** user runs `cerbo vault remove <id> --json`
- **THEN** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to vault active command
- **WHEN** user runs `cerbo vault active <id> --json`
- **THEN** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to page list command
- **WHEN** user runs `cerbo page list <vault_id> --json`
- **THEN** the output SHALL be a valid JSON array of page objects with fields: slug, title, path
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to page create command
- **WHEN** user runs `cerbo page create <vault_id> "Page Title" --json`
- **THEN** the output SHALL be a valid JSON object with field: slug (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to page read command
- **WHEN** user runs `cerbo page read <vault_id> <slug> --json`
- **THEN** the output SHALL be a valid JSON object with field: content (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to page write command
- **WHEN** user runs `cerbo page write <vault_id> <slug> "content" --json`
- **THEN** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to page delete command
- **WHEN** user runs `cerbo page delete <vault_id> <slug> --json`
- **THEN** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to page rename command
- **WHEN** user runs `cerbo page rename <vault_id> <slug> "New Title" --json`
- **THEN** the output SHALL be a valid JSON object with field: new_slug (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to attachment list command
- **WHEN** user runs `cerbo page attachment list <vault_id> <slug> --json`
- **THEN** the output SHALL be a valid JSON array of attachment objects with fields: filename, path, size (optional)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to attachment add command
- **WHEN** user runs `cerbo page attachment add <vault_id> <slug> /path/to/file --json`
- **THEN** the output SHALL be a valid JSON object with field: filename (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to attachment delete command
- **WHEN** user runs `cerbo page attachment delete <vault_id> <slug> <filename> --json`
- **THEN** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to index build command
- **WHEN** user runs `cerbo index build <vault_id> --json`
- **THEN** the output SHALL be a valid JSON object with field: success (boolean) and message (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to index backlinks command
- **WHEN** user runs `cerbo index backlinks <vault_id> <slug> --json`
- **THEN** the output SHALL be a valid JSON array of backlink objects with fields: slug, title
- **AND** no other text SHALL be printed to stdout

#### Scenario: Add --json flag to info command
- **WHEN** user runs `cerbo info --json`
- **THEN** the output SHALL be a valid JSON object with fields: config_dir (string), cache_dir (string), vaults (array of vault summary objects)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Watch command with --json flag
- **WHEN** user runs `cerbo watch --json`
- **THEN** each update event SHALL output a valid JSON object with fields: event (string), vault_id (string), message (string)
- **AND** no other text SHALL be printed to stdout

#### Scenario: Commands without --json flag remain unchanged
- **WHEN** user runs any command without `--json` flag
- **THEN** the output SHALL remain in the current human-readable format
- **AND** the behavior SHALL NOT change from current implementation

#### Scenario: Invalid JSON output SHALL NOT occur
- **WHEN** a command with `--json` flag encounters an error
- **THEN** the output SHALL be a valid JSON object with fields: error (boolean), message (string), code (optional integer)
- **AND** the exit code SHALL be non-zero
