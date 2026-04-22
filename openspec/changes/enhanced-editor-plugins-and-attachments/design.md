## Context
The Cerbo editor currently uses the base Carta markdown editor without any extensions. While functional, it lacks features common in modern wiki/PKM tools (syntax highlighting, attachments). Pages are stored as `<slug>/page.md`, which provides a natural location for page-specific assets.

## Goals / Non-Goals

**Goals:**
- Extend the editor with productivity plugins (code, emoji, anchor).
- Implement a robust attachment system for images and documents.
- Ensure attachments are accessible and manageable via both the UI and CLI.
- Maintain page portability by keeping assets within the page directory.

**Non-Goals:**
- Global asset library (all assets are page-scoped).
- Built-in image editing or PDF viewing (system default handlers or basic browser rendering).
- Cloud sync for attachments (out of scope for local-first core).

## Decisions

### 1. Attachment Storage Strategy
**Decision**: Store all attachments in `<vault>/<slug>/assets/`.
**Rationale**: This keeps the page and its assets together. When a page is deleted, the assets are deleted. When a page is moved/renamed, the assets move with it.
**Alternatives**:
- `vault/assets/<slug>/`: Makes page directories cleaner but breaks portability if only the page folder is copied.
- `vault/.assets/<hash>`: Content-addressable storage. Good for de-duplication but makes manual file management (outside Cerbo) impossible for users.

### 2. Carta Plugin Integration
**Decision**: Use official `@cartamd` plugins.
**Rationale**: Native compatibility with the existing editor, minimal configuration needed, and active maintenance.

### 3. Core-First Attachment Logic
**Decision**: Implement all file operations (add, list, delete) in `cerbo-core`.
**Rationale**: Allows the CLI to support the same features as the Tauri app, ensuring the tool remains powerful for power users and automation.

## Risks / Trade-offs

- **[Risk]**: Users might attach very large files, leading to slow vault operations or disk space issues.
  - **Mitigation**: Cerbo will not enforce limits initially, but the UI should display file sizes.
- **[Risk]**: Renaming a page might break links to attachments if not handled.
  - **Mitigation**: The `page_rename` function in `cerbo-core` must be updated to move the entire slug directory (which it currently does, but we must verify it handles the `assets` subfolder correctly).
- **[Risk]**: Duplicate filenames in `assets/`.
  - **Mitigation**: The system should either rename on conflict (e.g., `image_1.png`) or overwrite. For now, we will allow overwriting but notify the user in the UI.

## Migration Plan
1. Add dependencies to `package.json`.
2. Implement `attachment_*` functions in `core/src/page.rs`.
3. Add CLI subcommands to `cli/src/main.rs`.
4. Register Tauri commands in `src-tauri/src/main.rs` that wrap the core functions.
5. Create `AttachmentsPanel.svelte` and integrate it into `src/routes/+page.svelte`.
6. Configure Carta plugins in `src/lib/PageEditor.svelte`.
