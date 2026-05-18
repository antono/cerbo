## 1. Core: extend cerbo_core::page

- [ ] 1.1 Add `page_update_title(ctx, uuid, new_title)` to `core/src/page.rs` — updates `:title` in `meta.ttl` and replaces the first H1 in `page.md`
- [ ] 1.2 Route `page::page_write` through `links::page_write_with_links` instead of calling `object_write` directly
- [ ] 1.3 Export `page_update_title` from `core/src/lib.rs`

## 2. Tauri backend: page commands

- [ ] 2.1 Rewrite `src-tauri/src/page.rs` to import `cerbo_core::page` and delegate all commands to it (drop `cerbo_core::object::*` imports)
- [ ] 2.2 Remove inline `page_list` scan from `src-tauri/src/page.rs` — call `cerbo_core::page::page_list(ctx)` directly
- [ ] 2.3 Add `page_update_title` Tauri command to `src-tauri/src/page.rs`
- [ ] 2.4 Register `page_update_title` in the `invoke_handler` in `src-tauri/src/lib.rs`

## 3. Tauri backend: backlinks command

- [ ] 3.1 Change `backlinks_get` return type from `Vec<String>` to `Vec<BacklinkEntry>` in `src-tauri/src/index.rs`
- [ ] 3.2 Add `BacklinkEntry { uuid, title }` struct (serialisable) in `src-tauri/src/index.rs`
- [ ] 3.3 After calling `backrefs_read`, resolve each UUID by reading `ObjectMeta::read_from_file(meta.ttl)`; skip entries where `meta.ttl` is unreadable

## 4. Tauri backend: vault command

- [ ] 4.1 Rename the `slug` parameter to `uuid` in `vault_update_last_page` in `src-tauri/src/vault.rs` and the corresponding `core/src/vault.rs` function signature

## 5. Tauri backend: dead code removal

- [ ] 5.1 Delete `src-tauri/src/rename.rs`
- [ ] 5.2 Delete `src-tauri/src/slug.rs`
- [ ] 5.3 Confirm neither file is declared with `mod` in `lib.rs` (they are not — verify and leave clean)

## 6. Frontend: TypeScript types

- [ ] 6.1 Change `PageMeta.slug` → `PageMeta.uuid` in `src/lib/stores.svelte.ts`
- [ ] 6.2 Change `BacklinkEntry.slug` → `BacklinkEntry.uuid` in `src/lib/stores.svelte.ts`

## 7. Frontend: app state fields

- [ ] 7.1 Rename `app.currentSlug` → `app.currentUuid` in the `$state` object in `stores.svelte.ts`
- [ ] 7.2 Rename `app.renameSlug` → `app.renameUuid` in the `$state` object
- [ ] 7.3 Rename `app.confirmDeleteSlug` → `app.confirmDeleteUuid` in the `$state` object

## 8. Frontend: store functions

- [ ] 8.1 Update `pageSlugs()` helper → rename to `pageUuids()`, return `app.pages.map(p => p.uuid)`
- [ ] 8.2 Update `openPage(uuid)` — fix `page_read` invoke (drop `vaultId`; pass `uuid`), fix `vault_update_last_page` invoke (pass `uuid` not `slug`), update `loadBacklinks(uuid)` and `loadAttachments(uuid)` calls
- [ ] 8.3 Update `openVault` — fix last-page restore to match on `p.uuid`, fix index-page fallback to match on `p.title === 'Index'`
- [ ] 8.4 Update `savePage(uuid, content)` — fix `page_write` invoke (pass `uuid`, drop `vaultId`)
- [ ] 8.5 Update `createPage(title)` — fix `page_create` invoke (drop `vaultId`); call `openPage` with the returned UUID
- [ ] 8.6 Update `deletePage(uuid)` — fix `page_delete` invoke (drop `vaultId`); fix fallback `openPage` call to use `app.pages[0].uuid`
- [ ] 8.7 Replace `renamePage` with `updatePageTitle(uuid, newTitle)` — invoke `page_update_title`, reload pages, call `openPage(uuid)`
- [ ] 8.8 Update `triggerRename` and `triggerDelete` to use `renameUuid` / `confirmDeleteUuid`
- [ ] 8.9 Update `loadBacklinks(uuid)` invoke — pass `uuid` (was `slug`)
- [ ] 8.10 Update `loadAttachments(uuid)` invoke — pass `uuid` (was `slug`)
- [ ] 8.11 Remove `previewSlug` function (called `slug_from_title` which no longer exists)
- [ ] 8.12 Update `openNextPage` / `openPrevPage` to use `p.uuid` in `findIndex` and `openPage` calls
- [ ] 8.13 Update `goBack` / `goForward` history navigation — no field rename needed (already stores plain strings), but verify history is populated with UUIDs

## 9. Frontend: Svelte components

- [ ] 9.1 Update `PageList.svelte` — replace all `p.slug` / `currentSlug` references with `p.uuid` / `currentUuid`
- [ ] 9.2 Update `NewPageDialog.svelte` — remove slug preview (calls `previewSlug`); update `createPage` call
- [ ] 9.3 Update `RenamePageDialog.svelte` — call `updatePageTitle(renameUuid, newTitle)` instead of `renamePage`; remove slug preview
- [ ] 9.4 Update `ConfirmationDialog.svelte` — use `confirmDeleteUuid` instead of `confirmDeleteSlug`
- [ ] 9.5 Update `PageEditor.svelte` — replace all `currentSlug` references with `currentUuid`
- [ ] 9.6 Update `GlobalSearch.svelte` — navigate by `uuid` not `slug`
- [ ] 9.7 Search remaining Svelte components for `slug` references and fix any missed occurrences

## 10. Verification

- [ ] 10.1 Run `nix develop --command cargo check -p cerbo-desktop` — zero errors
- [ ] 10.2 Run TypeScript type-check (`nix develop --command bun run check`) — zero `slug` type errors
- [ ] 10.3 Manual smoke test: open vault → page list loads → open page → edit and save → backlinks update → rename page → delete page
- [ ] 10.4 Verify stale-slug recovery: set `last_open_page = "old-slug"` in `state.toml`, reopen vault, confirm graceful fallback to first/index page
