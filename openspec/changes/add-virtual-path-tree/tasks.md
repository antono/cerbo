# Tasks

## 1. Core: path updates

- [x] 1.1 Add `virtual_path` (serialised `virtualPath`) to `page::PageMeta` and fill it in `page_list`; verify a core unit test lists a page with `notes/rust` as `Some("notes/rust")` and a page without the predicate as `None`
- [x] 1.2 Implement `page::page_update_virtual_path(ctx, uuid, Option<String>)`: validate first, read/mutate/atomic-write `meta.ttl`, call `object::sync_index`; verify unit tests for move into folder, move to root (predicate gone), same path, each invalid form rejected without a write, unknown UUID, Source page allowed, `page.md` untouched
- [x] 1.3 Implement `page::folder_rename(ctx, old, new) -> FolderRenameReport` with `== prefix || starts_with(prefix + "/")` matching, per-page writes, collected failures, one `semantic::sync_objects` for all changed UUIDs; verify unit tests for nested rename, `notes` vs `notebook`, merge into existing folder, zero matches, invalid new prefix writes nothing, one unreadable `meta.ttl` does not stop the rest
- [x] 1.4 Verify the semantic index sees the new path: a test in `core/src/semantic/session.rs` style moves a page and runs `SELECT ?p WHERE { ?p cerbo:virtualPath "notes/rust" }` without a reindex

## 2. CLI

- [x] 2.1 Add `cerbo page move <UUID> [PATH] --root [--json]` with a clap `ArgGroup` (exactly one of PATH/--root); verify `cli_integration.rs` tests for text output, `--json` with null, invalid path exit code and stderr, both PATH and `--root` as usage error, unknown UUID
- [x] 2.2 Add top-level `cerbo folder rename <OLD> <NEW> [--json]`; verify integration tests for nested rename count, zero matches exits zero, invalid new prefix exits non-zero with no writes
- [x] 2.3 Extend `cerbo page list` text output with ` [path]` suffix and `--json` with `virtualPath`; verify the existing `test_page_list` still passes and a new test covers both a pathed and a root page
- [x] 2.4 Document `page move`, `folder rename` and the `page list` change in `cli/man/cerbo.md` and `docs/cli.md`; verify `nix build .#cerbo` succeeds and `man ./result/share/man/man1/cerbo.1` shows the new commands
- [x] 2.5 Update `.agents/skills/cerbo-cli/` with the two new commands; verify the skill text matches the man page wording

## 3. Tauri commands

- [x] 3.1 Add `page_move(uuid, virtualPath)` and `folder_rename(oldPrefix, newPrefix)` commands in `src-tauri/src/page.rs` via `off_main`, register in `lib.rs`; verify `nix build .#cerbo-desktop` succeeds and `invoke('page_list')` entries carry `virtualPath`
- [x] 3.2 Add `collapsed_folders: BTreeMap<String, Vec<String>>` (per vault id, serde default) to `UiSettings` and `merge_ui_settings`, plus a `ui_settings_set_collapsed_folders(vaultId, paths)` command; verify a core round-trip test reads an old `ui.toml` without the table and writes/reads the new one

## 4. Frontend: tree model and rendering

- [x] 4.1 Add `virtualPath` to `PageMeta`, a `buildPageTree(pages)` pure function in `src/lib/page-tree.ts` returning `TreeNode[]` (folders first, locale-aware case-insensitive sort), a `flattenVisible(tree, collapsed)` helper and a `folderPaths(pages)` helper; verify vitest tests for nesting, sort order, flat vault, `notes` vs `notebook`, collapsed subtree omitted, folder paths distinct and sorted
- [x] 4.2 Add `app.collapsedFolders` (Set, per active vault) with load on vault open, save on change with stale-path pruning, and ancestor expansion inside `openPage`; verify vitest tests for prune and for reveal-on-open
- [x] 4.3 Rewrite `PageList.svelte` to render flattened rows with depth indent, chevron, folder/page icons, `data-kind`/`data-path`/`data-uuid` attributes, active highlighting and scroll-into-view of the active row; keep hover actions on page rows; verify `bun run check` and a manual check that a vault without paths looks like the old list
- [x] 4.4 Extend the sidebar keyboard handler: j/k/arrows/Tab over visible rows (wrap), `h`/`ArrowLeft` collapse or go to parent, `l`/`ArrowRight` expand or enter, Enter/Space on folder toggles, Enter on page opens, `m` opens Move or Rename-folder; add the keys to `KeyboardHelp.svelte`; verify vitest tests for the row-stepping logic over a collapsed tree and the existing hotkey tests still pass

## 5. Frontend: editing the tree

- [x] 5.1 Extract `VirtualPathInput.svelte` from `NewPageDialog.svelte` with suggestions from `folderPaths(app.pages)` and a `validateVirtualPath` port of the core rules in `src/lib/virtual-path.ts`; verify vitest tests mirror the core validation cases and `NewPageDialog` still creates a page with a path
- [x] 5.2 Add `MovePageDialog.svelte` driven by `app.moveUuid`, with `movePage(uuid, path)` in the store calling `page_move` then `loadPages()` and expanding the target folder; wire the hover Move button and the `m` key; verify a vitest test for the store action and a manual check of move into new folder, move to root, invalid path disables confirm, backend error shown
- [x] 5.3 Add `RowMenu.svelte` context menu (right-click on a row; outside click and Escape close) with Open/Rename/Move/Delete for pages and Rename folder/New page here for folders; verify `bun run check` and a manual check in both themes
- [x] 5.4 Add `RenameFolderDialog.svelte` driven by `app.renameFolderPath`, with `renameFolder(old, new)` in the store calling `folder_rename`, showing `failed` entries in the error banner, and carrying over the collapsed state to the new path; verify a vitest test for the collapsed-state carry-over and a manual check of nested rename and rename-with-slash
- [x] 5.5 Make "New page here" open `NewPageDialog` with `app.newPagePath` prefilled; verify a manual check that the created page lands in the folder and opens in Write mode
- [x] 5.6 Add native drag-and-drop: `draggable` page rows, folder rows and a root drop zone as targets with a highlight class, no-op on page rows and on the current folder, expand the target after drop; verify a manual check of drop on folder, on collapsed folder, on root zone, on a page row (nothing) and `bun run check`

## 6. End-to-end and checks

- [x] 6.1 Add a desktop e2e case under `e2e/specs/` that creates a page in `notes/rust`, sees it nested in the sidebar, moves it to root through the dialog and sees the folder vanish; verify `e2e/run.sh` passes locally
- [x] 6.2 Run `cargo test --workspace`, `bun run check`, `bunx vitest run` inside `nix develop`, `nix build .#cerbo .#cerbo-desktop`, and `openspec validate add-virtual-path-tree --strict`; verify all pass
- [ ] 6.3 Manual pass on the built app with `test-vault`: tree rendering, collapse persistence across restart and across two vaults, keyboard navigation, move dialog, context menu, folder rename, drag-and-drop, light and dark themes; verify no console errors
  - Covered by `e2e/specs/vault/page-tree.e2e.ts` on the nix-built app (2026-11-04): tree rendering, collapse/expand by click and `h`/`l`, context menu, folder rename, move dialog. Still to check by hand: drag-and-drop (not scriptable in WebKitWebDriver), collapse persistence across a restart and across two vaults, both themes.

## Workflow follow-up

- Archive the change after review; the archive syncs the five delta specs into `openspec/specs/`.
- Fill in the `Purpose` placeholder of `cli-tooling` if archive leaves one.
