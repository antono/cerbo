# vault-auto-registration Specification

## Purpose
TBD - created by archiving change vault-auto-registration. Update Purpose after archive.
## Requirements
### Requirement: Auto-register CWD-discovered vault
When the CWD walk-up finds a vault root that is not present in either the manual registry (`vaults.toml`) or the auto registry (`vaults.auto.toml`), the system SHALL silently record it in `vaults.auto.toml` with a generated ID and a name derived from the directory name. This registration SHALL be idempotent: if the vault path is already in `vaults.auto.toml`, no write occurs.

#### Scenario: New vault discovered from CWD
- **WHEN** the CLI starts and CWD walk-up finds a `.cerbo/` directory not in any registry
- **THEN** the vault is added to `vaults.auto.toml` with a new UUID and name from the directory
- **THEN** no warning or output is produced
- **THEN** subsequent invocations do not add duplicate entries

#### Scenario: Already auto-registered vault
- **WHEN** the CLI starts and the discovered vault is already in `vaults.auto.toml`
- **THEN** no write occurs and the existing entry is unchanged

#### Scenario: Already manually registered vault
- **WHEN** the discovered vault path is in `vaults.toml`
- **THEN** no auto-registration occurs

### Requirement: Unified vault list with is_auto flag
The `vault_list` function in `cerbo_core::vault` SHALL return a single `Vec<Vault>` that merges entries from both `vaults.toml` (manual) and `vaults.auto.toml` (auto), with `is_auto: bool` set accordingly. Manual entries take precedence: if the same vault path appears in both files, only the manual entry is returned.

#### Scenario: Mixed registry
- **WHEN** `vault_list` is called and both files contain entries
- **THEN** the returned list contains all manual entries (is_auto=false) and all auto entries (is_auto=true) whose path does not duplicate a manual entry

#### Scenario: No auto-vaults file
- **WHEN** `vaults.auto.toml` does not exist
- **THEN** `vault_list` returns only manual entries without error

#### Scenario: Duplicate path in both files
- **WHEN** a vault path appears in both `vaults.toml` and `vaults.auto.toml`
- **THEN** only the manual entry appears in the unified list (auto entry is suppressed)

### Requirement: vault approve command
The system SHALL provide `cerbo vault approve <id>` that promotes an auto-registered vault to the manual registry. The command SHALL move the entry from `vaults.auto.toml` to `vaults.toml` (clearing `is_auto`). It SHALL error if the ID is not in the auto registry or if the path is already in the manual registry.

#### Scenario: Approve auto-registered vault
- **WHEN** user runs `cerbo vault approve <id>` and `<id>` is in `vaults.auto.toml`
- **THEN** the entry is appended to `vaults.toml` with `is_auto = false`
- **THEN** the entry is removed from `vaults.auto.toml`
- **THEN** the command succeeds with confirmation output

#### Scenario: Approve unknown ID
- **WHEN** user runs `cerbo vault approve <id>` and `<id>` is not in `vaults.auto.toml`
- **THEN** the command exits non-zero with an error message

#### Scenario: Approve already-manual vault
- **WHEN** user runs `cerbo vault approve <id>` and the vault's path is already in `vaults.toml`
- **THEN** the command exits non-zero indicating the vault is already manually registered

