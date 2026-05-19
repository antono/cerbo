## Why

The cerbo-desktop (Tauri + SvelteKit) was built against the old slug-based page model, but `core` has migrated to UUID object storage (`uuid-object-storage` spec). The frontend and Tauri command layer still use `slug` as the primary page identifier everywhere, causing a full frontend/backend mismatch.

## What Changes

- Replace `slug` with `uuid` as the page identifier in all frontend state, Tauri commands, and internal wiring
- Fix `page_write` to call `page_write_with_links` (currently bypasses backref tracking)
- Fix `backlinks_get` to return `{ uuid, title }` entries instead of raw UUID strings
- **BREAKING**: Remove `page_rename` slug-based command; replace with `page_update_title(uuid, newTitle)`
- Remove `slug_from_title` command (slug concept gone from desktop)
- Fix `vault_update_last_page` to store UUID (already stores any string; rename param for clarity)
- Implement stub Tauri commands: `cursor_position_save/load`, `attachment_upload`

## Capabilities

### New Capabilities
- `desktop-uuid-page-navigation`: Frontend navigates pages by UUID; `PageMeta` carries `{ uuid, title }`; all state, history, and URL tracking use UUID
- `desktop-backlinks-panel`: `backlinks_get` returns resolved `{ uuid, title }` entries; frontend renders them by UUID

### Modified Capabilities
- `page-crud`: Tauri `page_write` must route through `page_write_with_links`; `page_create`/`page_delete` drop `vaultId` param; rename replaces slug-based API with title-update-by-uuid
- `backlinks`: `backlinks_get` command now resolves raw UUID list to title+uuid pairs before returning to frontend

## Impact

- `src/lib/stores.svelte.ts` — pervasive rename of `slug` → `uuid` in state, types, and all `invoke()` call sites
- All Svelte components that reference `slug`, `currentSlug`, `renameSlug`, `confirmDeleteSlug`
- `src-tauri/src/page.rs` — `page_write`, param names throughout
- `src-tauri/src/vault.rs` — `vault_update_last_page` param rename
- `src-tauri/src/index.rs` — `backlinks_get` return type and resolution logic
- `src-tauri/src/rename.rs` + `src-tauri/src/lib.rs` — remove old rename command, add title-update command
- `src-tauri/src/slug.rs` + registration in `lib.rs` — remove `slug_from_title` command
