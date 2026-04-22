## Why

The current Rust backend is tightly coupled with the Tauri application,
preventing command-line usage and headless operation. Users want to perform wiki
operations (indexing, renaming, searching) directly from the terminal or via
automation scripts without launching the desktop GUI.

## What Changes

- **Crate Split**: Reorganize the Rust codebase into a workspace with three
  members:
  - `cerbo-core`: A library containing the domain logic (indexing, link
    resolution, rename cascades, page CRUD).
  - `cerbo-desktop`: The Tauri application (wrapper around `core`).
  - `cerbo`: A new CLI tool providing access to `core` functionality.
- **CLI Capabilities**: Implement a `cerbo` binary with:
  - Commands for managing vaults and pages.
  - A `watch` command to monitor vault changes and maintain the index.
  - Support for triggering rename cascades from the terminal.
- **Decoupling**: Remove `tauri` dependencies from core logic, replacing
  `AppHandle` usage with path-based or trait-based configuration.

## Capabilities

### New Capabilities

- `cli-tooling`: Command-line interface for all vault and page operations.
- `background-sync`: File system watcher for headless index maintenance.

### Modified Capabilities

- `vault-management`: Ensure vault registration and path resolution work
  identically across CLI and Desktop.
- `rename-cascade`: Ensure the rename cascade logic is available as a library
  function callable by both CLI and Desktop.

## Impact

- **Breaking**: Complete reorganization of `src-tauri`.
- **APIs**: Tauri commands will become thin wrappers around `cerbo-core`.
- **Dependencies**: Introduce `clap` for the CLI; `notify` and `walkdir` move to
  `core`.
