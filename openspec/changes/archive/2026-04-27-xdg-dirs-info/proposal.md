## Why

Currently, Cerbo uses `directories::ProjectDirs` for the CLI and Tauri API paths for the desktop app. These work but don't strictly follow the XDG Base Directory Specification. Using the `xdg` crate provides proper XDG compliance and includes features for querying directories. Additionally, there's no way for users to see where config/cache data is stored or what vaults are registered.

## What Changes

1. Add `xdg` crate dependency to `core`
2. Create `CoreContext` to use `xdg::BaseDirectories` for config/cache paths
3. Add `cerbo info` CLI command showing config dir, cache dir, vaults with page counts
4. Add `--info` flag to Tauri app showing same information
5. Remove unused `directories` crate from CLI

## Capabilities

### New Capabilities
- `xdg-paths`: Use XDG Base Directory Specification for storing config and cache data
- `cerbo-info`: CLI and desktop command to display config paths and vault information

### Modified Capabilities
- `vault-management`: The info command will query vault page counts (already part of CRUD)

## Impact

- **Dependencies**: Add `xdg` crate to `core/Cargo.toml`
- **Rust**: Update `CerboContext` creation in CLI and Tauri
- **CLI**: Add `info` command to CLI
- **Desktop**: Add `--info` flag handling