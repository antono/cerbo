## ADDED Requirements

### Requirement: vault list shows auto-registered vaults with (auto) marker
The `cerbo vault list` command SHALL display auto-registered vaults alongside manually registered vaults. Each auto-registered vault SHALL be visually distinguished with an `(auto)` marker appended to the vault name. In JSON output, each vault entry SHALL include an `is_auto` boolean field.

#### Scenario: Mixed vault list human output
- **WHEN** user runs `cerbo vault list` and both manual and auto vaults exist
- **THEN** manual vaults are shown without marker (e.g. `abc123: MyVault`)
- **THEN** auto vaults are shown with marker (e.g. `def456: notes (auto)`)

#### Scenario: Mixed vault list JSON output
- **WHEN** user runs `cerbo vault list --json`
- **THEN** each vault entry includes `"is_auto": true` or `"is_auto": false`

### Requirement: vault approve subcommand
The system SHALL provide `cerbo vault approve <ID> [--json]` as a `vault` subcommand that promotes an auto-registered vault to the manual registry.

#### Scenario: Successful promotion
- **WHEN** user runs `cerbo vault approve <id>` for an auto-registered vault
- **THEN** the vault appears in `cerbo vault list` without the `(auto)` marker
- **THEN** the vault no longer appears in the auto registry

#### Scenario: JSON output on approve
- **WHEN** user runs `cerbo vault approve <id> --json`
- **THEN** output is `{"success": true, "message": "..."}` on success

### Requirement: vault remove works on both registries
The system SHALL allow `cerbo vault remove <id>` to remove a vault from either the manual or auto registry, searching both files. The command SHALL succeed regardless of which file the vault is in.

#### Scenario: Remove manually registered vault
- **WHEN** user runs `cerbo vault remove <id>` for a vault in `vaults.toml`
- **THEN** the vault is removed from `vaults.toml`
- **THEN** the command succeeds

#### Scenario: Remove auto-registered vault
- **WHEN** user runs `cerbo vault remove <id>` for a vault in `vaults.auto.toml`
- **THEN** the vault is removed from `vaults.auto.toml`
- **THEN** the command succeeds
