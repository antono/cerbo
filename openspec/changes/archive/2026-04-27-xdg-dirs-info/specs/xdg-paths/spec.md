# XDG Paths

## Purpose

Use XDG Base Directory Specification for storing configuration and cache data.

## Requirements

### Requirement: XDG config directory
The system SHALL use `$XDG_CONFIG_HOME/cerbo` (default: `~/.config/cerbo`) for storing configuration files like `vaults.json`.

#### Scenario: Config directory location
- **WHEN** the application starts
- **THEN** the system SHALL resolve config directory from `$XDG_CONFIG_HOME` environment variable if set
- **OR** use `~/.config/cerbo` as default

#### Scenario: Config directory creation
- **WHEN** config directory does not exist
- **THEN** the system SHALL create it automatically

### Requirement: XDG cache directory
The system SHALL use `$XDG_CACHE_HOME/cerbo` (default: `~/.cache/cerbo`) for storing cache files like search indices.

#### Scenario: Cache directory location
- **WHEN** the application starts
- **THEN** the system SHALL resolve cache directory from `$XDG_CACHE_HOME` environment variable if set
- **OR** use `~/.cache/cerbo` as default

#### Scenario: Cache directory per-vault
- **WHEN** vault indexing is needed
- **THEN** the system SHALL store vault-specific cache at `$XDG_CACHE_HOME/cerbo/<vault-id>/index.json`