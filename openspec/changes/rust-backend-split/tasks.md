## 1. Workspace Setup

- [ ] 1.1 Create root `Cargo.toml` with `[workspace]` and members `core`, `cli`, and `src-tauri`.
- [ ] 1.2 Initialize `core/` as a library crate (`cerbo-core`).
- [ ] 1.3 Initialize `cli/` as a binary crate (`cerbo`).
- [ ] 1.4 Update `src-tauri/Cargo.toml` to be a member of the workspace and name the package `cerbo-desktop`.

## 2. Core Extraction

- [ ] 2.1 Extract domain logic from `src-tauri/src/` modules (`index`, `page`, `rename`, `slug`, `vault`, `paths`) into `core/src/`.
- [ ] 2.2 Refactor `cerbo-core` to be platform-agnostic, removing all `tauri` dependencies.
- [ ] 2.3 Replace `AppHandle` usage in `core` with a configuration struct or trait for path resolution (config/cache dirs).
- [ ] 2.4 Ensure `cerbo-core` unit tests pass independently.

## 3. Desktop Refactor

- [ ] 3.1 Update `src-tauri/Cargo.toml` to depend on `cerbo-core`.
- [ ] 3.2 Rewrite Tauri commands in `src-tauri/src/` to be thin wrappers around `cerbo-core` functions.
- [ ] 3.3 Re-implement the FS watcher in `cerbo-desktop` using `cerbo-core`'s indexing logic.
- [ ] 3.4 Verify the desktop application builds and functions as before.

## 4. CLI Implementation

- [ ] 4.1 Add `clap` and other CLI dependencies to `cli/Cargo.toml`.
- [ ] 4.2 Implement `vault` commands: `list`, `add`, `remove`, `active`.
- [ ] 4.3 Implement `page` commands: `list`, `create`, `read`, `write`, `delete`, `rename`.
- [ ] 4.4 Implement `index` command: `build`, `backlinks`.
- [ ] 4.5 Implement `watch` command: long-running process that monitors vaults and updates the index.
- [ ] 4.6 Verify `cerbo` CLI can trigger a full rename cascade.
