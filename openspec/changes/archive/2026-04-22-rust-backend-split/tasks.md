## 1. Workspace Setup

- [x] 1.1 Create root `Cargo.toml` with `[workspace]` and members `core`, `cli`, and `src-tauri`.
- [x] 1.2 Initialize `core/` as a library crate (`cerbo-core`).
- [x] 1.3 Initialize `cli/` as a binary crate (`cerbo`).
- [x] 1.4 Update `src-tauri/Cargo.toml` to be a member of the workspace and name the package `cerbo-desktop`.

## 2. Core Extraction

- [x] 2.1 Extract domain logic from `src-tauri/src/` modules (`index`, `page`, `rename`, `slug`, `vault`, `paths`) into `core/src/`.
- [x] 2.2 Refactor `cerbo-core` to be platform-agnostic, removing all `tauri` dependencies.
- [x] 2.3 Replace `AppHandle` usage in `core` with a configuration struct or trait for path resolution (config/cache dirs).
- [x] 2.4 Ensure `cerbo-core` unit tests pass independently.

## 3. Desktop Refactor

- [x] 3.1 Update `src-tauri/Cargo.toml` to depend on `cerbo-core`.
- [x] 3.2 Rewrite Tauri commands in `src-tauri/src/` to be thin wrappers around `cerbo-core` functions.
- [x] 3.3 Re-implement the FS watcher in `cerbo-desktop` using `cerbo-core`'s indexing logic.
- [x] 3.4 Verify the desktop application builds and functions as before.

## 4. CLI Implementation

- [x] 4.1 Add `clap` and other CLI dependencies to `cli/Cargo.toml`.
- [x] 4.2 Implement `vault` commands: `list`, `add`, `remove`, `active`.
- [x] 4.3 Implement `page` commands: `list`, `create`, `read`, `write`, `delete`, `rename`.
- [x] 4.4 Implement `index` command: `build`, `backlinks`.
- [x] 4.5 Implement `watch` command: long-running process that monitors vaults and updates the index.
- [x] 4.6 Verify `cerbo` CLI can trigger a full rename cascade.
