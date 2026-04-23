# CLI Version Flag

## ADDED Requirements

### Requirement: CLI version output
The `cerbo` CLI SHALL print its version when `--version` flag is provided.

#### Scenario: Print version flag
- **WHEN** user runs `cerbo --version`
- **THEN** the system prints the semver version string (e.g., `cerbo 0.1.0`)
- **AND** exits with code 0

#### Scenario: Version with subcommand
- **WHEN** user runs `cerbo --version vault list`
- **THEN** the system prints the semver version string
- **AND** exits with code 0