# Tasks

## 1. Typing latency

- [ ] 1.1 Create a 137 KB fixture page in `test-vault`, record a WebKit Inspector timeline (`bun run dev-debug`) typing 137 characters, and write per-keystroke cost by function into `design.md` Context; verify the numbers are reproducible on a second run
- [ ] 1.2 Stop rendering the hidden preview while in write mode (wrapper around Carta `Renderer` or self-rendered preview on tab switch); verify with a new trace that remark/rehype no longer runs during typing and preview still renders on switch
- [ ] 1.3 Cut overlay cost (larger `highlightDelay`, no speculative update or no overlay above a size threshold from 1.1); verify keydown→paint p95 ≤ 16 ms in the trace and syntax colours still appear after a pause
- [ ] 1.4 Limit autocomplete input scanning to the current line and compute caret coordinates in `requestAnimationFrame` only while open; verify with `vitest` unit tests for the trigger detection
- [ ] 1.5 Add an e2e check that types 137 characters into the fixture page and asserts the textarea value updates per key without loss; verify `bun run test:e2e` passes

## 2. Link picker on `[`

- [ ] 2.1 Extract trigger detection (`[`, `[[`, `(cerbo://`, exclusions for `!`, list markers, code) into a pure TS module with unit tests; verify `vitest` covers task-list and code-span cases
- [ ] 2.2 Precompute lowercase search keys (title, type, virtualPath) when `app.vaultObjects` changes and filter with prefix-first ordering; verify unit test on 13 700 synthetic objects runs under 1 ms per query
- [ ] 2.3 Render the search field showing the query, empty state "No objects found", and keep focus in the textarea; verify in the running app that Esc, `]` and space dismiss and keep plain text
- [ ] 2.4 Insert `[Title](cerbo://objects/<uuid>)` on Enter/Tab/click with caret after `)`; verify with an e2e test

## 3. Vault path in the selector

- [ ] 3.1 Show the absolute path beside the name in `.vault-item` with middle truncation and `title` tooltip in `src/routes/+layout.svelte`; verify visually with two same-named vaults and a long path, and in an e2e assertion that the path text is present

## 4. Drop into the editor

- [ ] 4.1 Spike: on Linux (WebKitGTK) with `dragDropEnabled: true`, check which drags reach the webview (OS file, browser image, text from another app); record results in `design.md` and adjust Decision 4 if needed
- [ ] 4.2 Core: add `attachment_add_bytes` writing via temp dir + rename; verify Rust unit tests for success, MIME detection and no partial object on failure (`cargo test -p cerbo-core`)
- [ ] 4.3 Tauri: implement `attachment_upload` with a raw bytes body on top of 4.2; verify a command-level test or manual call returns a UUID and creates the object
- [ ] 4.4 Frontend: handle `onDragDropEvent` file drops (path → `attachment_add`) and HTML5 fragment drops (→ `attachment_upload`), map drop position to a textarea offset with caret fallback; verify in the running app for PNG, PDF and multi-file drops
- [ ] 4.5 Insert `![name](cerbo://…)` for `image/*`, `[name](cerbo://…)` otherwise; verify the image renders in preview and the attachments panel lists it after save
- [ ] 4.6 Spike: on Linux, check which clipboard types the webview `paste` event exposes for a file copied in Nautilus/Dolphin, for a screenshot and for "Copy image" in Firefox/Chromium and GIMP; if file paths are missing, add a Rust clipboard-read command; record the result in `design.md`
- [ ] 4.7 Handle `paste` on the editor: image data → `attachment_upload`, copied files (`file://` URIs) → `attachment_add`, text-only → default paste; insert the reference at the caret; verify in the running app that a copied PDF, a screenshot and a browser "Copy image" appear in the page's attachments panel, plus an e2e test that pastes a fixture image
- [ ] 4.8 Drop indicator while dragging, error toast and no reference on failure; replace the `@cartamd/plugin-attachment` upload callback with the same path; verify an e2e test that drops a fixture file and checks `page.md` and the panel

## 5. Restore caret and scroll

- [ ] 5.1 Core: store `{line, column, scrollTop}` per vault/page in `cursor.toml` in the config dir, with pruning; verify Rust unit tests for save/load/prune
- [ ] 5.2 Tauri: implement `cursor_position_save`/`cursor_position_load` (add `scrollTop`) on top of 5.1; verify they round-trip in a test
- [ ] 5.3 Frontend: save on preview toggle, page switch, vault switch, window close and 1 s after edits; restore caret and scroll on entering write mode, removing the scroll reset on `uuid` change; extend `cursor-position.test.ts` and verify in the running app across page switches and an app restart
- [ ] 5.4 Add an e2e test: edit at line N, switch page, return, assert caret line; verify `bun run test:e2e` passes

## 6. Integration

- [ ] 6.1 Run `bun run check`, `vitest`, `cargo test`, `nix build .#cerbo .#cerbo-desktop` and the e2e suite; verify all pass

## Workflow follow-up

- Archive the change after review and sync the delta specs into `openspec/specs/`.
