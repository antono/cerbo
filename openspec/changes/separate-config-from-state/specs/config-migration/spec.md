# Config Migration

## ADDED Requirements

### Requirement: Migration from JSON on startup
The system SHALL detect and migrate legacy JSON config to TOML format when old config file is found.

#### Scenario: Old config exists
- **WHEN** system starts and `$XDG_CONFIG_DIR/cerbo/vaults.json` exists
- **THEN** system SHALL migrate data to TOML format and delete old JSON file

#### Scenario: New config exists
- **WHEN** system starts and no old config exists
- **THEN** migration skipped, normal startup continues

#### Scenario: Migration creates config.toml
- **WHEN** migration runs with old vaults in JSON
- **THEN** config.toml created with vaults array

#### Scenario: Migration creates state.toml
- **WHEN** migration runs with active_vault_id in JSON
- **THEN** state.toml created with active_vault_id and per-vault last_open_page

#### Scenario: Migration deletes old config
- **WHEN** migration successfully writes TOML files
- **THEN** old vaults.json deleted from filesystem

#### Scenario: Migration handles missing optional fields
- **WHEN** old config missing active_vault_id or last_open_page
- **THEN** migration proceeds with null/empty values

#### Scenario: Migration fails to write TOML
- **WHEN** migration cannot write TOML (permissions)
- **THEN** old config preserved, error logged, system continues with defaults

### Requirement: Migration logs actions
The system SHALL log migration actions for debugging.

#### Scenario: Migration runs
- **WHEN** migration executes
- **THEN** log entry indicates migration completed and what files were created

## ADDED Requirements

### Requirement: Migration code marked for removal
The migration code SHALL include a TODO comment marking removal after v0.3.0.

```rust
// TODO: Remove this migration code after v0.3.0 release
// The old vaults.json format was deprecated in v0.2.0.
// Migration allowed users to upgrade smoothly.
// After v0.3.0, assume all users have migrated.
```

#### Scenario: TODO comment present
- **WHEN** developer reviews migration.rs
- **THEN** they see TODO marking removal date (v0.3.0)