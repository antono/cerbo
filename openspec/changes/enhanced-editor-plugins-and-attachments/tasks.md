## 1. Core Implementation (cerbo-core)

- [ ] 1.1 Implement `attachment_list` in `core/src/page.rs` to scan `<slug>/assets/`
- [ ] 1.2 Implement `attachment_add` in `core/src/page.rs` with directory creation and atomic copy
- [ ] 1.3 Implement `attachment_delete` in `core/src/page.rs`
- [ ] 1.4 Add unit tests for attachment operations in `core/src/page.rs`
- [ ] 1.5 Verify `page_rename` correctly moves the `assets/` subfolder (as part of the slug directory move)

## 2. CLI Extension (cerbo-cli)

- [ ] 2.1 Add `Attachment` subcommands to `Cli` and `PageCommands` in `cli/src/main.rs`
- [ ] 2.2 Implement CLI handlers for listing, adding, and deleting attachments

## 3. Tauri & Frontend Setup

- [ ] 3.1 Install `@cartamd/plugin-code`, `@cartamd/plugin-emoji`, `@cartamd/plugin-attachment`, `@cartamd/plugin-anchor`
- [ ] 3.2 Add Tauri commands to `src-tauri/src/page.rs` to wrap core attachment functions
- [ ] 3.3 Register new Tauri commands in `src-tauri/src/main.rs`
- [ ] 3.4 Import and configure plugins in `src/lib/PageEditor.svelte`

## 4. UI Components & Layout

- [ ] 4.1 Create `src/lib/AttachmentsPanel.svelte` to list and delete assets
- [ ] 4.2 Implement attachment upload handler for the Carta plugin in `PageEditor.svelte`
- [ ] 4.3 Update `src/routes/+page.svelte` to include `AttachmentsPanel` in the right sidebar
- [ ] 4.4 Refine right sidebar CSS to handle multiple panels (Backlinks + Attachments)
