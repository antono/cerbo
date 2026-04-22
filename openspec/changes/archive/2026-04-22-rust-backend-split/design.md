## Context

Cerbo currently has a single Rust package in `src-tauri` that contains all
backend logic. This logic is tightly coupled with Tauri's `AppHandle` and
platform-specific path management. To support a CLI and headless operation, we
need to extract the core domain logic into a platform-agnostic library.

## Goals / Non-Goals

**Goals:**

- Decouple domain logic (indexing, renames, page CRUD, vault management) from
  the Tauri framework.
- Create a reusable `cerbo-core` crate as the single source of truth for wiki
  operations.
- Create a `cerbo` tool for terminal-based workflow, automation, and headless
  operation.
- Maintain existing Tauri functionality by refactoring it to use `cerbo-core`.
- Desktop application should be named 'cerbo-desktop'.
- Provide a CLI watcher for background indexing.

**Non-Goals:**

- Changing the frontend (Svelte) logic or state management.
- Modifying the underlying data format (Markdown folders).
- Implementing a persistent background daemon (the watcher will be a CLI
  command).

## Decisions

### 1. Workspace Structure

We will move to a Cargo workspace at the project root.

- **Decision**: Create a root `Cargo.toml` defining a workspace with members
  `core`, `cli`, and `desktop` (the existing `src-tauri`).
- **Rationale**: This is the idiomatic way to manage multiple binaries and
  libraries in a single Rust project.

### 2. Decoupling Path Management

The core logic currently depends on `tauri::AppHandle` to resolve config and
cache directories.

- **Decision**: Refactor `core` functions to accept explicit path configurations
  or a `CerboContext` struct.
- **Rationale**: CLI and Desktop have different ways of determining "home"
  directories. `core` should not care about the platform runner.

### 3. Rename Cascade and Indexing

These are currently triggered by Tauri commands.

- **Decision**: Move the logic for link extraction, index building, and rename
  cascades into `core`. Both `desktop` and `cli` will call these shared
  functions.
- **Rationale**: Avoids code duplication and ensures behavior consistency.

### 4. Watcher Implementation

- **Decision**: The `core` library will provide a platform-agnostic indexing
  service. The `desktop` app will continue to use its existing `notify`-based
  watcher, while the `cli` will implement its own using the same `core` indexing
  logic.
- **Rationale**: Watcher state management (e.g., Tauri's managed state) is
  platform-specific, but the _action_ (rebuilding index) is core logic.

## Risks / Trade-offs

- [Risk] → Breaking the Tauri build process due to path changes.
  - Mitigation: Carefully update `tauri.conf.json` and ensure `src-tauri`
    remains a valid Tauri project directory.
- [Risk] → Index corruption if CLI and Desktop run simultaneously.
  - Mitigation: Since the index is a cache of the filesystem, corruption is
    low-risk, but we will consider simple file-based locking if needed.
- [Risk] → Divergent behavior between CLI and Desktop.
  - Mitigation: All domain-affecting logic MUST reside in `core`.
