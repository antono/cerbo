# Proposal: Separate Config from State

## Why

Cerbo currently stores both persistent configuration and ephemeral UI state in a single JSON config file. This conflates two different concerns: user preferences that should persist across machines/sessions, and per-vault state that is transient. Moving ephemeral state to XDG_CACHE_DIR and separating UI settings allows proper caching behavior, enables caching per-vault state, and cleans up the config architecture.

## What Changes

- **BREAKING**: Config format changes from JSON to TOML
- Store persistent config in `$XDG_CONFIG_DIR/cerbo/vaults.toml`
- Store ephemeral UI state in `$XDG_CACHE_DIR/cerbo/state.toml` (active vault, last open page, window state)
- Store per-vault cache in vault cache directory (search index, rendered content cache)
- Add migration: detect old JSON config at `$XDG_CONFIG_DIR/cerbo/config.json` and migrate to TOML
- Remove migration code after v0.3.0 release (add TODO comment marking removal date)

## Capabilities

### New Capabilities

- **config-migration**: Auto-detect and migrate legacy JSON config to TOML format when found
- **per-vault-cache**: Store ephemeral cache data alongside vault (search index, render cache)
- **ui-state**: Separate UI state (active vault, last page, window bounds) from user config

### Modified Capabilities

- **config**: Change storage format (JSON → TOML) and split persistent vs ephemeral data

## Impact

- Core config module needs refactoring
- CLI and desktop app both affected (share config logic)
- Migration runs once on startup if old config detected
