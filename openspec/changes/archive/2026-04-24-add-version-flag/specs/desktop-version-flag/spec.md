# Desktop Version Flag

## ADDED Requirements

### Requirement: Desktop version output
The `cerbo-desktop` CLI binary SHALL print its version when `--version` flag is provided.

#### Scenario: Print version flag
- **WHEN** user runs `cerbo-desktop --version`
- **THEN** the system prints the semver version string (e.g., `cerbo-desktop 0.1.0`)
- **AND** exits with code 0

#### Scenario: Version with GUI args
- **WHEN** user runs `cerbo-desktop --version`
- **THEN** the system prints the semver version string
- **AND** exits with code 0
- **AND** the GUI is NOT launched