## 1. Core Implementation (cerbo-core)

- [x] 1.1 Implement `attachment_list` in `core/src/page.rs` to scan `<slug>/assets/`
- [x] 1.2 Implement `attachment_add` in `core/src/page.rs` with directory creation and atomic copy
- [x] 1.3 Implement `attachment_delete` in `core/src/page.rs`
- [x] 1.4 Add unit tests for attachment operations in `core/src/page.rs`
- [x] 1.5 Verify `page_rename` correctly moves the `assets/` subfolder (as part of the slug directory move)

## 2. CLI Extension (cerbo-cli)

- [x] 2.1 Add `Attachment` subcommands to `Cli` and `PageCommands` in `cli/src/main.rs`
- [x] 2.2 Implement CLI handlers for listing, adding, and deleting attachments

## 3. Tauri & Frontend Setup

- [x] 3.1 Install `@cartamd/plugin-code`, `@cartamd/plugin-emoji`, `@cartamd/plugin-attachment`, `@cartamd/plugin-anchor`
- [x] 3.2 Add Tauri commands to `src-tauri/src/page.rs` to wrap core attachment functions
- [x] 3.3 Register new Tauri commands in `src-tauri/src/main.rs`
- [x] 3.4 Import and configure plugins in `src/lib/PageEditor.svelte`

## 4. UI Components & Layout

- [x] 4.1 Create `src/lib/AttachmentsPanel.svelte` to list and delete assets
- [x] 4.2 Implement attachment upload handler for the Carta plugin in `PageEditor.svelte`
- [x] 4.3 Update `src/routes/+page.svelte` to include `AttachmentsPanel` in the right sidebar
- [x] 4.4 Refine right sidebar CSS to handle multiple panels (Backlinks + Attachments)
